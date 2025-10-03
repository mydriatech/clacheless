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

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

mod config;

use clacheless::DistributedCache;
use std::process::ExitCode;
use tokio::signal::unix::SignalKind;
use tokio::signal::unix::signal;

/// Application main entrypoint.
fn main() -> ExitCode {
    if let Err(e) = init_logger() {
        println!("Failed to initialize logging: {e:?}");
        return ExitCode::FAILURE;
    }
    // Defaults to using one thread per core when no limit is set.
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run_async(
            &config::address_template(),
            config::local_node_id(),
            config::cache_item_time_to_live_micros(),
            "0.0.0.0",
            8080,
        ))
}

/// Initialize the logging system and apply filters.
fn init_logger() -> Result<(), log::SetLoggerError> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .filter(Some("actix_server::builder"), log::LevelFilter::Warn)
        .filter(Some("h2"), log::LevelFilter::Info)
        .filter(Some("tower"), log::LevelFilter::Info)
        .filter(Some("hyper_util"), log::LevelFilter::Info)
        .filter(Some("actix_server::server"), log::LevelFilter::Warn)
        .write_style(env_logger::fmt::WriteStyle::Auto)
        .target(env_logger::fmt::Target::Stdout)
        .is_test(false)
        .parse_env(
            env_logger::Env::new()
                .filter("LOG_LEVEL")
                .write_style("LOG_STYLE"),
        )
        .try_init()
}

/// Async code entry point.
pub async fn run_async(
    address_template: &str,
    local_node_id: u32,
    cache_item_ttl_micros: u64,
    http_bind_address: &str,
    http_bind_port: u16,
) -> ExitCode {
    let dc = DistributedCache::new(address_template, local_node_id, cache_item_ttl_micros).await;
    let dc_future = dc.run();
    let app_future =
        clacheless_api_rest::rest_api::run_http_server(&dc, http_bind_address, http_bind_port);
    let signals_future = block_until_signaled();
    let res = tokio::select! {
        res = app_future => {
            log::trace!("app_future finished");
            res
        },
        res = dc_future => {
            log::trace!("dc_future finished");
            res.map_err(|e|
                Box::new(e) as Box<dyn std::error::Error>
            )
        },
        _ = signals_future => {
            log::trace!("signals_future finished");
            Ok(())
        },
    }
    .map_err(|e| log::error!("{e}"));
    if res.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Block until SIGTERM or SIGINT is recieved.
async fn block_until_signaled() {
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigterm.recv() => {
            log::trace!("SIGTERM recieved.")
        },
        _ = sigint.recv() => {
            log::trace!("SIGINT recieved.")
        },
    };
}
