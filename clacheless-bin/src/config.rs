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

//! Configuration parsing.

/// Return the address template where the literal String `ORDINAL` will be
/// replaced by the target node's id.
pub fn address_template() -> String {
    env_or_default(
        "CLACHELESS_ADDR_TEMPLATE",
        "statefulsetname-ORDINAL.headlessservicename.namespace.svc:9090",
    )
}

/// Return the local node's ID.
pub fn local_node_id() -> u32 {
    let pod_name = env_or_default("POD_NAME", "clacheless-0");
    clacheless::util::extract_ordinal_from_string(&pod_name).unwrap_or(0)
}

/// Return for how many microseconds a checked item will be kept.
pub fn cache_item_time_to_live_micros() -> u64 {
    env_or_default("CLACHELESS_TTL", "3600")
        .parse()
        .unwrap_or(3600)
        * 1_000_000
}

/// Get environment variable by name or return a default value if the variable
/// isn't set.
fn env_or_default(name: &str, default_value: &str) -> String {
    std::env::var(name)
        .inspect_err(|_e| log::warn!("Missing env.{name} -> using default value '{default_value}'"))
        .unwrap_or(default_value.to_string())
}
