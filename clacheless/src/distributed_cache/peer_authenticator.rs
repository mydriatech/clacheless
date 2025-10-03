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

//! Simplistic authentication for distributed cache communication.

use std::sync::Arc;
use std::sync::OnceLock;
use tyst::Tyst;
use tyst::traits::mac::MacKey;
use tyst::traits::mac::ToMacKey;

static AUTHENTICATOR: OnceLock<Arc<PeerAuthenticator>> = OnceLock::new();

/** Provide short lived authentication tokens to prove that instances belong
to the cache.

The scope of this protection is to prevent access to the gRPC API from other
entities in the cluster and not having to rely on network isolation.

This *does not* protect against replay attacks for the validity of the tokens
nor provide any guarantees of message authenticity or origin.

Tokens are derived as `b64url(time|HMAC-SHA3-256(key,time))` where only this
app's containers should have access to the `key`.

`/secrets/dc/key` is expected to hold a 136 bytes base64 encoded
String with the key.
*/
pub struct PeerAuthenticator {
    secret: Box<dyn MacKey>,
}

impl PeerAuthenticator {
    /// Recommended header name
    pub const HEADER_NAME: &str = "internal-auth";
    /// Authorization ticket validity duration
    const TOKEN_VALIDITY: u64 = 1_000_000;

    fn new() -> Arc<Self> {
        // Read secret from file (136 bytes for HMAC-SHA3-256)
        Arc::new(Self {
            secret: Self::get_secret("/secrets/dc/key").to_mac_key(),
        })
    }

    /// Shared secret
    fn get_secret(filename: &str) -> Vec<u8> {
        match std::fs::read_to_string(std::path::PathBuf::from(filename)) {
            Ok(content) => match tyst::encdec::base64::decode(&content) {
                Ok(secret) => {
                    if log::log_enabled!(log::Level::Debug) {
                        log::debug!("Peer auth secret is {} bytes long.", secret.len());
                    }
                    return secret;
                }
                Err(e) => {
                    log::warn!("Failed to parse '{filename}': {e}");
                }
            },
            Err(e) => {
                log::warn!("Failed to parse '{filename}': {e}");
            }
        }
        log::info!(
            "An ephemeral secret will be generated due to previous error. This is only acceptable for testing."
        );
        Tyst::instance().prng_get_random_bytes(None, 136)
    }

    /// Return instance.
    pub fn instance() -> Arc<Self> {
        AUTHENTICATOR.get_or_init(Self::new).clone()
    }

    /// Get short-lived peer authentication token.
    pub fn create_token(&self) -> Option<String> {
        let now_micros = crate::time::get_timestamp_micros();
        let mut time_and_mac = now_micros.to_be_bytes().to_vec();
        self.create_mac(&time_and_mac)
            .map(|mac| {
                time_and_mac.extend_from_slice(&mac);
                time_and_mac
            })
            .map(|time_and_mac| tyst::encdec::base64::encode_url(&time_and_mac, false))
    }

    /// Validate peer authentication token.
    pub fn is_token_valid(&self, b64urlenc: &str) -> bool {
        let time_and_mac = tyst::encdec::base64::decode_url(b64urlenc).unwrap_or_default();
        if time_and_mac.is_empty() {
            return false;
        }
        let mut time_bytes = [0u8; 8];
        time_bytes.copy_from_slice(&time_and_mac[0..8]);
        let ts_micros = u64::from_be_bytes(time_bytes);
        let now_micros = crate::time::get_timestamp_micros();
        let mac = self.create_mac(&time_and_mac[0..8]).unwrap_or_default();
        mac.eq(&time_and_mac[8..]) && ts_micros > now_micros - Self::TOKEN_VALIDITY
    }

    /// Create a HMAC-SHA3-256 message authenctication code of message.
    fn create_mac(&self, message: &[u8]) -> Option<Vec<u8>> {
        tyst::Tyst::instance()
            .macs()
            .by_oid(&tyst::encdec::oid::as_string(
                tyst::oids::mac::HMAC_SHA3_256,
            ))
            .map(|mut mac_impl| mac_impl.mac(self.secret.as_ref(), message))
    }
}
