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

//! Local sequence number generation.

use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

/// Local sequence number generator.
pub struct LocalSequence {
    node_id: u64,
    seq: AtomicU64,
}

impl LocalSequence {
    /// Return a new instance.
    pub fn new(local_node_id: u64) -> Self {
        Self {
            node_id: local_node_id,
            seq: AtomicU64::default(),
        }
    }

    /// Return the local node id.
    pub fn node_id(&self) -> u64 {
        self.node_id
    }

    /// Return `true` if this generator has generated any sequence number.
    pub fn has_been_pulled(&self) -> bool {
        self.seq.load(Ordering::Relaxed) > 0
    }

    /// Return the current (last generated) sequence number.
    pub fn current(&self) -> u64 {
        self.seq.load(Ordering::Relaxed)
    }

    /// Return a fresh sequence number.
    pub fn generate_next(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::Relaxed) + 1
    }
}
