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

//! GRPC server for inter-Pod communication.

use super::DistributedCache;
use super::peer_authenticator::PeerAuthenticator;
use crate::ClachelessError;
use crate::ClachelessErrorKind;
use crate::proto::stateshare::InitStateTransferReply;
use crate::proto::stateshare::InitStateTransferRequest;
use crate::proto::stateshare::PutCacheEntryReply;
use crate::proto::stateshare::PutCacheEntryRequest;
use crate::proto::stateshare::StateViewUpdateReply;
use crate::proto::stateshare::StateViewUpdateRequest;
use crate::proto::stateshare::state_share_server::StateShare;
use crate::proto::stateshare::state_share_server::StateShareServer;
use std::sync::Arc;
use tonic::Request;
use tonic::Response;
use tonic::Status;
use tonic::async_trait;
use tonic::transport::Server;

/// gRPC server implementation.
struct StateShareImpl {
    dc: Arc<DistributedCache>,
}

#[async_trait]
impl StateShare for StateShareImpl {
    /// Receive a cache entry from remote node.
    async fn put_cache_entry(
        &self,
        request: Request<PutCacheEntryRequest>,
    ) -> Result<Response<PutCacheEntryReply>, Status> {
        let ur = request.into_inner();
        self.dc
            .put_raw_from_remote_origin(
                ur.key,
                ur.object_bytes,
                ur.this_update_micros,
                ur.expires,
                ur.origin_node_id,
                ur.origin_node_update_seq,
            )
            .await
            .map_err(|e| Status::unknown(e.to_string()))?;
        Ok(tonic::Response::new(PutCacheEntryReply::default()))
    }

    /// Receive remote node's view of the cluster.
    async fn state_view_update(
        &self,
        request: Request<StateViewUpdateRequest>,
    ) -> Result<Response<StateViewUpdateReply>, Status> {
        let svr = request.into_inner();
        if log::log_enabled!(log::Level::Trace) {
            log::trace!("Got state update: {svr:?}");
        }
        self.dc
            .on_state_view(svr.sender_node_ordinal, svr.view)
            .await;
        Ok(tonic::Response::new(StateViewUpdateReply {}))
    }

    /// Receive a request for a state transfer
    async fn init_state_transfer(
        &self,
        request: Request<InitStateTransferRequest>,
    ) -> Result<Response<InitStateTransferReply>, Status> {
        let istr = request.into_inner();
        if log::log_enabled!(log::Level::Trace) {
            log::trace!("Got state transfer request: {istr:?}");
        }
        self.dc
            .transfer_state(istr.reciever_node_ordinal, istr.data_origin_id_and_baseline)
            .await
            .map_err(|e| Status::unknown(e.to_string()))?;
        Ok(tonic::Response::new(InitStateTransferReply {}))
    }
}

/// Run gRPC server.
///
/// This will not return for as long the server is running.
pub async fn run_grpc_server(
    dc: &Arc<DistributedCache>,
    bind_port: u16,
) -> Result<(), ClachelessError> {
    let addr = format!("0.0.0.0:{bind_port}").parse().unwrap();
    let state_share_impl = StateShareImpl { dc: Arc::clone(dc) };
    log::info!("Clacheless gRPC service is listening on {addr}");
    Server::builder()
        .add_service(StateShareServer::with_interceptor(
            state_share_impl,
            authorization_interceptor,
        ))
        .serve(addr)
        .await
        .map_err(|e| {
            ClachelessErrorKind::Unspecified
                .error_with_msg(format!("Failed to start gRPC server: {e}"))
        })
}

/// Validate token of request ensure that it is part of the same cluster.
fn authorization_interceptor(req: Request<()>) -> Result<Request<()>, Status> {
    if log::log_enabled!(log::Level::Trace) {
        log::trace!("(server) authorization_interceptor: {req:?}");
    }
    match req.metadata().get(PeerAuthenticator::HEADER_NAME) {
        Some(token)
            if PeerAuthenticator::instance().is_token_valid(token.to_str().unwrap_or_default()) =>
        {
            Ok(req)
        }
        other => {
            if log::log_enabled!(log::Level::Trace) {
                log::trace!("Failed to validate peer authentication token: {other:?}");
            }
            Err(Status::unauthenticated("No valid auth token."))
        }
    }
}
