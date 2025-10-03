/*
    Copyright 2025 MydriaTech AB

    Licensed under the Apache License 2.0 with Free world makers exception
    1.0.0 (the "License"); you may not use this file except in compliance with
    the License. You should have obtained a copy of the License with the source
    or binary distribution in file named

        LICENSE-Apache-2.0-with-FWM-Exception-1.0.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.
*/

//! Distributed cache.

mod cluster_view;
mod grpc_client;
mod grpc_server;
mod local_cache;
mod peer_authenticator;

use self::cluster_view::ClusterStateView;
use self::grpc_client::GrpcClient;
use self::local_cache::LocalCache;
use crate::ClachelessError;
use crate::ClachelessErrorKind;
use crossbeam_skiplist::SkipMap;
use crossbeam_skiplist::map::Entry;
use std::collections::HashMap;
use std::sync::Arc;

/** Distributed cache between `Pod`s in a `StatefulSet`.

[Self] maintains connectivity to other `Pod`s in the `StatefulSet` and holds
the local copy of the distributed cache.

Node identifiers are unique (for 136 years) and calculated at starup as
`local_node_id = now_seconds & 0xffff_ffff << 32 | node_ordinal`.

*/
pub struct DistributedCache {
    address_template: String,
    local_node_ordinal: u32,
    cache_item_ttl_micros: u64,
    local_node_id: u64,
    known_node_ordinals_with_last_seen: SkipMap<u32, u64>,
    local_cache: Arc<LocalCache>,
    cluster_view: Arc<ClusterStateView>,
}

impl DistributedCache {
    const STATE_BROADCAST_INTERVAL_MICROS: u64 = 2_000_000;
    const ALIVE_MARGIN_MICROS: u64 = 500_000;
    const MAX_AGE_BEFORE_IGNORED_MICROS: u64 =
        Self::STATE_BROADCAST_INTERVAL_MICROS + Self::ALIVE_MARGIN_MICROS;

    /// Return a new instance.
    ///
    /// `address_template` should be in the form a `fqdn:port` with the literal
    /// string `ORDINAL` present.
    pub async fn new(
        address_template: &str,
        local_node_ordinal: u32,
        cache_item_ttl_micros: u64,
    ) -> Arc<Self> {
        let now_seconds = crate::time::get_timestamp_micros() / 1_000_000;
        let local_node_id = (now_seconds & 0xffff_ffff) << 32 | u64::from(local_node_ordinal);
        Arc::new(Self {
            address_template: address_template.to_string(),
            local_node_ordinal,
            cache_item_ttl_micros,
            local_node_id,
            known_node_ordinals_with_last_seen: SkipMap::default(),
            local_cache: LocalCache::new().await,
            cluster_view: ClusterStateView::new(local_node_id),
        })
        .init()
        .await
    }

    async fn init(self: Arc<Self>) -> Arc<Self> {
        let self_clone = Arc::clone(&self);
        tokio::spawn(async move { self_clone.remove_expired_other_nodes().await });
        self
    }

    /// Start publishing local state to other nodes and start local gRPC server.
    ///
    /// This function will not return for as long as the server is running.
    pub async fn run(self: &Arc<Self>) -> Result<(), ClachelessError> {
        let self_clone = Arc::clone(self);
        tokio::spawn(async move { self_clone.notify_other_nodes().await });
        let port = self.get_address_template_port();
        grpc_server::run_grpc_server(self, port).await
    }

    fn get_address_for_node_ordinal(&self, node_ordinal: u32) -> String {
        self.address_template
            .replacen("ORDINAL", &node_ordinal.to_string(), 1)
    }

    /// Extract gRPC address port from template or default to 9000.
    fn get_address_template_port(&self) -> u16 {
        self.address_template
            .match_indices(':')
            .next_back()
            .map(|(last_dash_index, _)| self.address_template.split_at(last_dash_index + 1).1)
            .and_then(|ordinal_string| {
                ordinal_string
                    .parse::<u16>()
                    .inspect_err(|e| log::debug!("Failed to parse ordinal '{ordinal_string}': {e}"))
                    .ok()
            })
            .unwrap_or(9000)
    }

    /// Periodically notify all other nodes about this node's ClusterStateView.
    async fn notify_other_nodes(self: &Arc<Self>) {
        loop {
            for node_ordinal in 0..=self.get_highest_known_node_ordinal() {
                if node_ordinal != self.local_node_ordinal {
                    let address = self.get_address_for_node_ordinal(node_ordinal);
                    if log::log_enabled!(log::Level::Trace) {
                        log::trace!("Pushing view to '{address}'.");
                    }
                    let self_clone = Arc::clone(self);
                    let _res = tokio::spawn(async move {
                        let grpc_client = GrpcClient::new(&address).await?;
                        grpc_client
                            .push_state_view(
                                self_clone.local_node_ordinal,
                                self_clone.cluster_view.as_map().await,
                            )
                            .await
                            .inspect_err(|e| log::debug!("Push failed: {e}"))
                    });
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_micros(
                Self::STATE_BROADCAST_INTERVAL_MICROS,
            ))
            .await;
        }
    }

    /// Periodically check if other nodes has disappeared.
    async fn remove_expired_other_nodes(self: &Arc<Self>) {
        loop {
            let now_micros = crate::time::get_timestamp_micros();
            for entry in self.known_node_ordinals_with_last_seen.iter() {
                if *entry.value() < now_micros - Self::MAX_AGE_BEFORE_IGNORED_MICROS {
                    entry.remove();
                    log::info!(
                        "Lost connectivity to distributed cache node with ordinal '{}'.",
                        entry.key()
                    );
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_micros(
                Self::STATE_BROADCAST_INTERVAL_MICROS,
            ))
            .await;
        }
    }

    /// Invoked when a remote node pushed its view of the cluster to this node.
    ///
    /// If the remote node has more up to date data than this node, a state
    /// transfer will be requested from the remote node for the delta.
    async fn on_state_view(&self, sender_ordinal: u32, view: HashMap<u64, u64>) {
        log::trace!("Got state update: {view:?}");
        let now_micros = crate::time::get_timestamp_micros();
        let is_new = self
            .known_node_ordinals_with_last_seen
            .get(&sender_ordinal)
            .as_ref()
            .map(Entry::value)
            .filter(|last_seen_micros| {
                **last_seen_micros >= now_micros - Self::MAX_AGE_BEFORE_IGNORED_MICROS
            })
            .is_none();
        self.known_node_ordinals_with_last_seen
            .insert(sender_ordinal, now_micros);
        let data_origin_id_and_baseline = self
            .cluster_view
            .get_out_of_sync_node_id_and_baselines(view)
            .await;
        if !data_origin_id_and_baseline.is_empty() {
            log::debug!(
                "This node is lagging behind and need a state transfer: {data_origin_id_and_baseline:?}"
            );
            let address = self.get_address_for_node_ordinal(sender_ordinal);
            if let Ok(grpc_client) = GrpcClient::new(&address)
                .await
                .inspect_err(|e| log::info!("Failed to connect: {e}"))
            {
                grpc_client
                    .request_state_transfer(self.local_node_ordinal, data_origin_id_and_baseline)
                    .await
                    .inspect_err(|e| log::info!("Failed to request state transfer: {e}"))
                    .ok();
            }
        }
        if is_new {
            log::info!("New distributed cache node with ordinal '{sender_ordinal}' detected.");
        }
    }

    /// Return the highest known `node_ordinal` that is confirmed to be alive
    /// (has checked in).
    fn get_highest_known_node_ordinal(&self) -> u32 {
        let last_seen_threshold =
            crate::time::get_timestamp_micros() - Self::MAX_AGE_BEFORE_IGNORED_MICROS;
        *self
            .known_node_ordinals_with_last_seen
            .iter()
            .filter(|entry| *entry.value() > last_seen_threshold)
            .inspect(|v| {
                if log::log_enabled!(log::Level::Trace) {
                    log::trace!("other nodes entry: {v:?}")
                }
            })
            .last()
            .as_ref()
            .map(Entry::key)
            .unwrap_or(&self.local_node_ordinal)
    }

    /// Initiate transfer of more up to date local state to the remote.
    pub async fn transfer_state(
        self: &Arc<Self>,
        reciever_node_ordinal: u32,
        data_origin_id_and_baseline: HashMap<u64, u64>,
    ) -> Result<(), ClachelessError> {
        let address = self.get_address_for_node_ordinal(reciever_node_ordinal);
        let grpc_client = GrpcClient::new(&address)
            .await
            .inspect_err(|e| log::debug!("Failed to connect: {e}"))?;
        let self_clone = Arc::clone(self);
        tokio::spawn(async move {
            for fcde in self_clone.local_cache.iter(&data_origin_id_and_baseline) {
                grpc_client
                    .send_update(
                        fcde.key,
                        fcde.ce.this_update_micros,
                        fcde.ce.expires_micros,
                        fcde.ce.object_bytes.to_vec(),
                        fcde.ce.origin_node_id,
                        fcde.ce.origin_node_update_seq,
                    )
                    .await
                    .inspect_err(|e| log::info!("Failed to send update: {e}"))
                    .ok();
            }
        });
        Ok(())
    }

    /// Send cache item to all known nodes.
    async fn broadcast_update(
        &self,
        key: String,
        this_update_micros: u64,
        expires: u64,
        object_bytes: Vec<u8>,
        origin_node_id: u64,
        update_seq: u64,
    ) -> Result<(), ClachelessError> {
        for node_ordinal in 0..=self.get_highest_known_node_ordinal() {
            if node_ordinal != self.local_node_ordinal {
                let address = self.get_address_for_node_ordinal(node_ordinal);
                let key = key.to_owned();
                let object_bytes = object_bytes.to_owned();
                let _res = tokio::spawn(async move {
                    let grpc_client = GrpcClient::new(&address).await?;
                    grpc_client
                        .send_update(
                            key,
                            this_update_micros,
                            expires,
                            object_bytes,
                            origin_node_id,
                            update_seq,
                        )
                        .await
                });
            }
        }
        Ok(())
    }

    /// Insert raw cache item as recieved during state transfer and update local
    /// cluster view.
    async fn put_raw_from_remote_origin(
        &self,
        cache_key: String,
        cache_value: Vec<u8>,
        this_update_micros: u64,
        expires_micros: u64,
        origin_node_id: u64,
        origin_node_update_seq: u64,
    ) -> Result<(), ClachelessError> {
        if log::log_enabled!(log::Level::Debug) {
            log::debug!(
                "Got update for key '{cache_key}' created on node_id {origin_node_id} (ordinal: {}).",
                origin_node_id & 0xffff_ffff
            );
        }
        self.local_cache.put(
            cache_key,
            cache_value,
            this_update_micros,
            origin_node_id,
            origin_node_update_seq,
            expires_micros,
        )?;
        self.cluster_view
            .on_recieved_cache_entry_from_other(origin_node_id, origin_node_update_seq)
            .await;
        Ok(())
    }

    /// Insert item in cache and broadcast update to all other known nodes.
    pub async fn put_bytes(
        &self,
        cache_key: &str,
        cache_value: &[u8],
    ) -> Result<(), ClachelessError> {
        let update_seq = self.cluster_view.next_local_update_seq();
        let this_update_micros = crate::time::get_timestamp_micros();
        let expires = this_update_micros + self.cache_item_ttl_micros;
        self.broadcast_update(
            cache_key.to_owned(),
            this_update_micros,
            expires,
            cache_value.to_owned(),
            self.local_node_id,
            update_seq,
        )
        .await
        .inspect_err(|e| log::debug!("Failed to broadcast update: {e}"))
        .ok();
        self.local_cache.put(
            cache_key.to_string(),
            cache_value.to_vec(),
            this_update_micros,
            self.local_node_id,
            update_seq,
            expires,
        )
    }

    /// Insert item in cache and broadcast update to all other known nodes.
    pub async fn put_string(
        &self,
        cache_key: &str,
        cache_value: &str,
    ) -> Result<(), ClachelessError> {
        self.put_bytes(cache_key, cache_value.as_bytes()).await
    }

    /// Get object bytes from cache.
    pub fn get_bytes(&self, cache_key: &str) -> Result<Arc<Vec<u8>>, ClachelessError> {
        self.local_cache.get(cache_key)
    }

    /// Get string object from cache.
    pub fn get_string(&self, cache_key: &str) -> Result<String, ClachelessError> {
        let cached_content = self.get_bytes(cache_key)?;
        String::from_utf8(cached_content.to_vec()).map_err(|e| {
            ClachelessErrorKind::Malformed.error_with_msg(format!(
                "Entry for {cache_key} was not an UTF-8 string: {e}"
            ))
        })
    }
}
