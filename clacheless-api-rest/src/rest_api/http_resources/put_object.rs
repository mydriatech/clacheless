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

//! API resource for storing a cached item by key.

use crate::rest_api::AppState;
use crate::rest_api::common::ApiErrorMapper;
use actix_web::Error;
use actix_web::HttpRequest;
use actix_web::HttpResponse;
use actix_web::error;
use actix_web::http::StatusCode;
use actix_web::put;
use actix_web::web;
use actix_web::web::Data;
use actix_web::web::Path;
use actix_web::web::Payload;
use futures::StreamExt;

/// Limit payload size to 5 MiB.
const MAX_DOCUMENT_SIZE: usize = 5 * 1024 * 1024;

/// Storing a cached item by key.
#[utoipa::path(
    tag = "cache",
    params(
        ("key", description = "Cache key."),
    ),
    responses(
        (status = 204, description = "No content. Successfully cached item."),
        (status = 400, description = "Bad Request."),
        (status = 500, description = "Internal server error."),
    ),
)]
#[put("/cache/{cache_key}")]
pub async fn put_object(
    app_state: Data<AppState>,
    path: Path<String>,
    payload: Payload,
    http_request: HttpRequest,
) -> Result<HttpResponse, Error> {
    let cache_key = path.into_inner();
    let content_length_estimate = assert_declared_content_length(&http_request, MAX_DOCUMENT_SIZE)?;
    let raw_cache_value = read_full_body_text(content_length_estimate, payload).await?;
    app_state
        .dc
        .put_string(&cache_key, &raw_cache_value)
        .await
        .map_err(ApiErrorMapper::from_error)?;
    Ok(HttpResponse::build(StatusCode::NO_CONTENT).finish())
}

/// Assert that the declared content-length header (if present) is within the
/// max_size limit.
fn assert_declared_content_length(
    http_request: &HttpRequest,
    max_size: usize,
) -> Result<usize, Error> {
    let content_length_estimate = http_request
        .headers()
        .get("content-length")
        .and_then(|header_value| header_value.to_str().ok())
        .and_then(|header_value_str| header_value_str.parse::<usize>().ok())
        .unwrap_or(1024);
    if content_length_estimate > max_size {
        Err(error::ErrorBadRequest("overflow"))?
    } else {
        Ok(content_length_estimate)
    }
}

async fn read_full_body_text(
    content_length_estimate: usize,
    mut payload: Payload,
) -> Result<String, Error> {
    let mut body = web::BytesMut::with_capacity(content_length_estimate);
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        // limit max size of in-memory payload
        if (body.len() + chunk.len()) > MAX_DOCUMENT_SIZE {
            Err(error::ErrorBadRequest(format!(
                "Message body exceeded {MAX_DOCUMENT_SIZE} bytes."
            )))?;
        }
        body.extend_from_slice(&chunk);
    }
    std::str::from_utf8(&body.freeze())
        .map_err(|e| error::ErrorBadRequest(format!("Message body was not valid UTF-8: {e}")))
        .map(str::to_string)
}
