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

//! REST API server and resources.

mod http_resources {
    //! API resources

    pub mod get_object;
    pub mod put_object;
}
mod common {
    //! Common RESP API resources and utils.

    mod api_error_mapper;

    pub use api_error_mapper::*;
}

use actix_web::App;
use actix_web::HttpResponse;
use actix_web::HttpServer;
use actix_web::Responder;
use actix_web::get;
use actix_web::http::header::ContentType;
use actix_web::web;
use clacheless::DistributedCache;
use std::sync::Arc;
use tyst_api_rest_health::AppHealth;
use tyst_api_rest_health::health_resources;
use utoipa::OpenApi;

/// Number of parallel requests the can be served for each assigned CPU core.
const WORKERS_PER_CORE: usize = 1024;

/// Shared state between requests.
#[derive(Clone)]
struct AppState {
    dc: Arc<DistributedCache>,
}

/// Simple health check that gets the provider instance.
pub struct AppHealthImpl {
    _app: Arc<DistributedCache>,
}
impl AppHealthImpl {
    fn with_app(app: &Arc<DistributedCache>) -> Arc<dyn AppHealth> {
        Arc::new(Self {
            _app: Arc::clone(app),
        })
    }
}
impl AppHealth for AppHealthImpl {
    fn is_health_started(&self) -> bool {
        true
    }
    fn is_health_ready(&self) -> bool {
        true
    }
    fn is_health_live(&self) -> bool {
        true
    }
}

/// Run HTTP server.
pub async fn run_http_server(
    dc: &Arc<DistributedCache>,
    bind_address: &str,
    bind_port: u16,
) -> Result<(), Box<dyn core::error::Error>> {
    let workers = std::thread::available_parallelism()
        .map(|non_zero| non_zero.get())
        .unwrap_or(1);
    let max_connections = WORKERS_PER_CORE * workers;
    log::info!(
        "API described by http://{bind_address}:{bind_port}/openapi.json allows {max_connections} concurrent connections."
    );
    let app_state: AppState = AppState { dc: Arc::clone(dc) };
    let app_data = web::Data::<AppState>::new(app_state);
    let app_health = web::Data::<Arc<dyn AppHealth>>::new(AppHealthImpl::with_app(dc));

    HttpServer::new(move || {
        let scope = web::scope("/api/v1")
            .service(get_openapi)
            .service(http_resources::get_object::get_object)
            .service(http_resources::put_object::put_object);
        App::new()
            .app_data(app_data.clone())
            .app_data(app_health.clone())
            .service(web::redirect("/openapi", "/api/v1/openapi.json"))
            .service(web::redirect("/openapi.json", "/api/v1/openapi.json"))
            .service(scope)
            .service(health_resources::health)
            .service(health_resources::health_live)
            .service(health_resources::health_ready)
            .service(health_resources::health_started)
    })
    .workers(workers)
    .backlog(u32::try_from(max_connections / 2).unwrap()) // Default is 2048
    .worker_max_blocking_threads(max_connections)
    .max_connections(max_connections)
    .bind_auto_h2c((bind_address, bind_port))?
    .disable_signals()
    .shutdown_timeout(5) // Default 30
    .run()
    .await?;
    Ok(())
}

/// Serve Open API documentation.
#[get("/openapi.json")]
async fn get_openapi() -> impl Responder {
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(openapi_as_string())
}

/// Get the OpenAPI definition as a pretty JSON String.
pub fn openapi_as_string() -> String {
    #[derive(OpenApi)]
    #[openapi(
        // Use Cargo.toml as source for the "info" section
        paths(
            http_resources::get_object::get_object,
            http_resources::put_object::put_object,
            health_resources::health,
            health_resources::health_live,
            health_resources::health_ready,
            health_resources::health_started,
        )
    )]
    struct ApiDoc;
    ApiDoc::openapi().to_pretty_json().unwrap()
}
