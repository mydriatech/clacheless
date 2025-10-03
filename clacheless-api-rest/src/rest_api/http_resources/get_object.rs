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

//! API resource for retrieving a cached item by key.

use crate::rest_api::AppState;
use crate::rest_api::common::ApiErrorMapper;
use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::get;
use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::web::Path;

/// Retrieve a cached item by key.
#[utoipa::path(
    tag = "cache",
    params(
        ("key", description = "Cache key."),
    ),
    responses(
        (
            status = 200,
            description = "Return the cached object.",
            content_type = "application/json",
        ),
        (
            status = 404,
            description = "No cached item with the key was found.",
        ),
        (status = 500, description = "Internal server error."),
    ),
)]
#[get("/cache/{key}")]
pub async fn get_object(
    app_state: Data<AppState>,
    path: Path<String>,
) -> Result<HttpResponse, Error> {
    let cache_key = path.into_inner();
    let object = app_state
        .dc
        .get_string(&cache_key)
        .inspect_err(|e| log::info!("Request for '{cache_key}' failed: {e}"))
        .map_err(ApiErrorMapper::from_error)?;
    Ok(HttpResponse::build(StatusCode::OK).body(object))
}
