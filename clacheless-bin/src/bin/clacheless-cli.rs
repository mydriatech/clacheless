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

//! REST API CLI for Clacheless.

use reqwest::StatusCode;
use std::process::ExitCode;

/// Basic CLI that can be extended later.
#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    if let Err(e) = init_logger() {
        println!("Failed to initialize logging: {e}");
        return ExitCode::FAILURE;
    }
    let mut args = std::env::args();
    //let app_version = "todo";
    let cli_name = args.next().unwrap_or_default();
    match args.next().as_deref() {
        Some("get") => {
            if let Some(cache_key) = args.next() {
                let base_url = args.next().unwrap_or("http://localhost:8080".to_string());
                if let Some(res) = get_cache_item(&base_url, &cache_key).await {
                    log::info!("{res:?}");
                    return ExitCode::SUCCESS;
                } else {
                    log::info!("No item with key '{cache_key}' was found.");
                    return ExitCode::FAILURE;
                }
            }
        }
        Some("put") => {
            if let Some(cache_key) = args.next()
                && let Some(cache_value) = args.next()
            {
                let base_url = args.next().unwrap_or("http://localhost:8080".to_string());
                if put_cache_item(&base_url, &cache_key, cache_value).await {
                    return ExitCode::SUCCESS;
                }
            }
        }
        Some(_other) => {}
        None => {}
    }
    log::info!(
        "{cli_name} - Ceso REST CLI

Usage:
    {cli_name} get <key> [base_url]
    {cli_name} put <key> <value> [base_url]

Example
    {cli_name} get some_key http://localhost:8080
    "
    );
    ExitCode::FAILURE
}

fn init_logger() -> Result<(), log::SetLoggerError> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .filter(Some("hyper_util"), log::LevelFilter::Info)
        .filter(Some("reqwest"), log::LevelFilter::Info)
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

/// Invoke REST API and load item from cache.
pub async fn get_cache_item(base_url: &str, cache_key: &str) -> Option<String> {
    let url = format!("{base_url}/api/v1/cache/{cache_key}");
    if log::log_enabled!(log::Level::Debug) {
        log::debug!("GET '{url}'");
    }
    match reqwest::get(&url).await {
        Ok(response) => match response.status() {
            StatusCode::OK => {
                return response
                    .text()
                    .await
                    .inspect_err(|e| log::warn!("Failed parsing response from '{url}': {e}"))
                    .ok();
            }
            StatusCode::NOT_FOUND => {
                // No need to log any additional info.
            }
            _other_status => {
                log::info!("Unexpected response status from '{url}': {response:?}");
            }
        },
        Err(e) => {
            log::warn!("Request to '{url}' failed: {e}");
        }
    }
    None
}

/// Invoke REST API and store item in cache.
pub async fn put_cache_item(base_url: &str, cache_key: &str, cache_value: String) -> bool {
    let url = format!("{base_url}/api/v1/cache/{cache_key}");
    if log::log_enabled!(log::Level::Debug) {
        log::debug!("PUT '{url}'");
    }
    let client = reqwest::Client::new();
    match client.put(&url).body(cache_value).send().await {
        Ok(response) => match response.status() {
            StatusCode::NO_CONTENT => {
                log::debug!("Ok");
                return true;
            }
            _other_status => {
                log::info!("Unexpected response status from '{url}': {response:?}");
            }
        },
        Err(e) => {
            log::warn!("Request to '{url}' failed: {e}");
        }
    }
    false
}
