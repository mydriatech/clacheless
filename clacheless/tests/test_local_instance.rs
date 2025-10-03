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

//! Integration tests of [InterPodCache].

use clacheless::DistributedCache;
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn run_local_instance() {
    let dc = DistributedCache::new("clacheless-ORDINAL.local:9000", 0, 30_000_000).await;
    let dc_clone = Arc::clone(&dc);
    tokio::spawn(async move { dc_clone.run().await });
    let cache_key = "cache_key";
    let cache_value = "cache_value";
    dc.put_string(cache_key, cache_value)
        .await
        .expect("Failed to update local-only cache.");
    let read_result = dc
        .get_string(&cache_key)
        .expect("Locally cached item should always be available.");
    assert_eq!(read_result, cache_value);
}
