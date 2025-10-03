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

//! Mapper of app errors to Actix-web [Error].

use actix_web::Error;
use actix_web::error;
pub use clacheless::ClachelessError;
use clacheless::ClachelessErrorKind;

/// Mapper of app errors to Actix-web [Error].
pub struct ApiErrorMapper {}

impl ApiErrorMapper {
    /// Return REST API [Error] from library defined error.
    pub fn from_error<E: AsRef<ClachelessError>>(e: E) -> Error {
        let e = e.as_ref();
        if log::log_enabled!(log::Level::Debug) {
            log::debug!("Will respond with error. kind: {} msg: {e}", e.kind());
        }
        match e.kind() {
            ClachelessErrorKind::Malformed => {
                // HTTP 400
                error::ErrorBadRequest(e.to_string())
            }
            ClachelessErrorKind::NotFound => {
                // HTTP 404
                error::ErrorNotFound(e.to_string())
            }
            _other => {
                // HTTP 500
                error::ErrorInternalServerError(e.to_string())
            }
        }
    }
}
