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

//! Local copy of the distributed cache.

use crate::ClachelessError;
use crate::ClachelessErrorKind;
use crossbeam_skiplist::SkipMap;
use crossbeam_skiplist::map::Entry;
use std::collections::HashMap;
use std::sync::Arc;

/// Cached object and meta data.
pub struct CacheEntry {
    /// Time the cache entry was first recieved at one of the cluster nodes.
    pub this_update_micros: u64,
    /// Node identifier where the cache entry was first recieved.
    pub origin_node_id: u64,
    /// The unique seqence number for cache entry on the node where it was first
    /// recieved.
    pub origin_node_update_seq: u64,
    /// Expiration date of the cache entry in epoch microseconds.
    pub expires_micros: u64,
    /// Raw bytes of the cached object.
    pub object_bytes: Arc<Vec<u8>>,
}

/// [CacheEntry] and the cached item's lookup key.
pub struct CacheEntryAndKey {
    /// Lookup key the cache entry is referenced by.
    pub key: String,
    pub ce: Arc<CacheEntry>,
}

/// Lock-free local copy of the distributed cache.
pub struct LocalCache {
    cache: SkipMap<String, Arc<CacheEntry>>,
}

impl LocalCache {
    /// Return a new instance.
    pub async fn new() -> Arc<Self> {
        Arc::new(Self {
            cache: SkipMap::default(),
        })
        .purge_expired()
        .await
    }

    // Background task to purge expired items from time to time.
    async fn purge_expired(self: Arc<Self>) -> Arc<Self> {
        let ret = Arc::clone(&self);
        tokio::spawn(async move {
            loop {
                let now_micros = crate::time::get_timestamp_micros();
                let mut count = 0;
                self.cache
                    .iter()
                    .filter(|entry| entry.value().expires_micros < now_micros)
                    .for_each(|entry| {
                        entry.remove().then(|| {
                            count += 1;
                        });
                    });
                if count > 0 {
                    log::info!("Purged {count} expired items from cache.");
                }
                tokio::time::sleep(tokio::time::Duration::from_micros(30_000_000)).await;
            }
        });
        ret
    }

    /// Return an iterator over all cached items that are non-expired and
    /// more up to date than the provided cluster view.
    ///
    /// Items are sorted by update origin node's update sequence to allow
    /// state transfer to send oldest items first.
    pub fn iter(
        &self,
        data_origin_id_and_baseline: &HashMap<u64, u64>,
    ) -> impl Iterator<Item = CacheEntryAndKey> {
        let now_micros = crate::time::get_timestamp_micros();
        /*
        To send the oldest entries first we need to sort by
        `origin_node_update_seq`.

        Since this implies storing everythign that needs sorting into mem, we
        collect only the `origin_node_update_seq` and `key`.

        Later during iteration the full cache entry is looked up on demand.
        */
        let mut key_by_update_seq = self
            .cache
            .iter()
            .filter_map(move |entry| {
                let ce = Arc::clone(entry.value());
                data_origin_id_and_baseline
                    .get(&ce.origin_node_id)
                    .is_some_and(|baseline| {
                        if log::log_enabled!(log::Level::Trace) {
                            log::trace!("ce.this_update_micros: {}, baseline: {baseline}, ce.expires_micros: {}, now_micros: {now_micros}", ce.this_update_micros, ce.expires_micros);
                        }
                        ce.this_update_micros > *baseline && ce.expires_micros > now_micros
                    })
                    .then_some((entry.key().to_owned(),ce.origin_node_update_seq))
            })
            .collect::<Vec<_>>();
        key_by_update_seq.sort_by_key(|(_key, origin_node_update_seq)| *origin_node_update_seq);
        key_by_update_seq.into_iter().filter_map(|(key, _)| {
            self.cache.get(&key).map(|entry| CacheEntryAndKey {
                key: entry.key().to_owned(),
                ce: Arc::clone(entry.value()),
            })
        })
    }

    /// Get non-expired cache item.
    pub fn get(&self, cache_key: &str) -> Result<Arc<Vec<u8>>, ClachelessError> {
        self.cache
            .get(cache_key)
            .as_ref()
            .map(Entry::value)
            .filter(|cde| cde.expires_micros >= crate::time::get_timestamp_micros())
            .map(|cde| Arc::clone(&cde.object_bytes))
            .ok_or_else(|| {
                ClachelessErrorKind::NotFound.error_with_msg(format!("No entry for {cache_key}."))
            })
    }

    /// Insert item in cache if it is newer than the existing one.
    pub fn put(
        &self,
        cache_key: String,
        cache_value: Vec<u8>,
        this_update_micros: u64,
        origin_node_id: u64,
        origin_node_update_seq: u64,
        expires_micros: u64,
    ) -> Result<(), ClachelessError> {
        self.cache.compare_insert(
            cache_key,
            Arc::new(CacheEntry {
                this_update_micros,
                origin_node_id,
                origin_node_update_seq,
                expires_micros,
                object_bytes: Arc::new(cache_value),
            }),
            |old_cde| old_cde.this_update_micros < this_update_micros,
        );
        Ok(())
    }
}
