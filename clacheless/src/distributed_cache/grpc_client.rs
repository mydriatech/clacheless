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

//! GRPC client for inter-Pod communication.

use super::peer_authenticator::PeerAuthenticator;
use crate::ClachelessError;
use crate::ClachelessErrorKind;
use crate::proto::stateshare::InitStateTransferRequest;
use crate::proto::stateshare::PutCacheEntryRequest;
use crate::proto::stateshare::StateViewUpdateRequest;
use crate::proto::stateshare::state_share_client::StateShareClient;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::Request;
use tonic::Status;
use tonic::metadata::MetadataValue;
use tonic::service::interceptor::InterceptedService;
use tonic::transport::Channel;

/// `tonic` interceptor function type alias.
type TonicInterceptorFn = fn(Request<()>) -> Result<Request<()>, Status>;

/// GRPC client for inter-Pod communication.
pub struct GrpcClient {
    client: Mutex<StateShareClient<InterceptedService<Channel, TonicInterceptorFn>>>,
    address: String,
}

impl GrpcClient {
    /// Return a new instance.
    ///
    /// `address` should only include fqdn and port.
    pub async fn new(address: &str) -> Result<Arc<Self>, ClachelessError> {
        let endpoint_string = format!("http://{address}");
        let channel = Channel::from_shared(endpoint_string)
            .map_err(|e| {
                ClachelessErrorKind::Connection
                    .error_with_msg(format!("Failed to parse gRPC address '{address}': {e}"))
            })?
            .connect()
            .await
            .map_err(|e| {
                ClachelessErrorKind::Connection.error_with_msg(format!(
                    "Failed to connect to gRPC address '{address}': {e}"
                ))
            })?;
        let client = StateShareClient::with_interceptor(
            channel,
            Self::authorization_interceptor as TonicInterceptorFn,
        );
        Ok(Arc::new(Self {
            client: Mutex::new(client),
            address: address.to_owned(),
        }))
    }

    /// Add token to request to prove that it is part of the same cluster.
    fn authorization_interceptor(mut req: Request<()>) -> Result<Request<()>, Status> {
        if log::log_enabled!(log::Level::Trace) {
            log::trace!("authorization_interceptor");
        }
        if let Some(token) = PeerAuthenticator::instance().create_token() {
            req.metadata_mut().insert(
                PeerAuthenticator::HEADER_NAME,
                token.parse::<MetadataValue<_>>().unwrap(),
            );
            if log::log_enabled!(log::Level::Trace) {
                log::trace!("(client) authorization_interceptor: {req:?}");
            }
        }
        Ok(req)
    }

    /// Request a state tranfer from the remote node.
    pub async fn request_state_transfer(
        &self,
        reciever_node_ordinal: u32,
        data_origin_id_and_baseline: HashMap<u64, u64>,
    ) -> Result<(), ClachelessError> {
        let request = Request::new(InitStateTransferRequest {
            reciever_node_ordinal,
            data_origin_id_and_baseline,
        });
        let mut client = self.client.lock().await;
        let response = client.init_state_transfer(request).await.map_err(|e| {
            ClachelessErrorKind::Connection.error_with_msg(format!(
                "Requesting state transfer from '{}' failed: {e}",
                self.address
            ))
        })?;
        if log::log_enabled!(log::Level::Trace) {
            log::trace!("request_state_transfer response: {response:?}");
        }
        Ok(())
    }

    /// Send a cache entry update to the remote node.
    pub async fn send_update(
        &self,
        key: String,
        this_update_micros: u64,
        expires: u64,
        object_bytes: Vec<u8>,
        origin_node_id: u64,
        origin_node_update_seq: u64,
    ) -> Result<(), ClachelessError> {
        let request = Request::new(PutCacheEntryRequest {
            key,
            this_update_micros,
            expires,
            object_bytes,
            origin_node_id,
            origin_node_update_seq,
        });
        let mut client = self.client.lock().await;
        let response = client.put_cache_entry(request).await.map_err(|e| {
            ClachelessErrorKind::Connection.error_with_msg(format!(
                "Sending cache entry update to '{}' failed: {e}",
                self.address
            ))
        })?;
        if log::log_enabled!(log::Level::Trace) {
            log::trace!("send_update response: {response:?}");
        }
        Ok(())
    }

    /// Send the local nodes cluster view to the remote.
    pub async fn push_state_view(
        &self,
        sender_node_ordinal: u32,
        view: HashMap<u64, u64>,
    ) -> Result<(), ClachelessError> {
        let request = Request::new(StateViewUpdateRequest {
            sender_node_ordinal,
            view,
        });
        let mut client = self.client.lock().await;
        let response = client.state_view_update(request).await.map_err(|e| {
            ClachelessErrorKind::Connection.error_with_msg(format!(
                "Pushing state view to '{}' failed: {e}",
                self.address
            ))
        })?;
        if log::log_enabled!(log::Level::Trace) {
            log::trace!("push_state_view response: {response:?}");
        }
        Ok(())
    }
}
