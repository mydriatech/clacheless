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

//! Local view of another node.

use std::sync::Arc;
use tokio::sync::Mutex;

/// Latest known sequence number of a remote node and how far the local node
/// has synchronized.
#[derive(Default)]
struct KnownSequences {
    /// Known baseline sequence number where the local node has recieved all
    /// available updates from the remote.
    baseline_seq: u64,
    /// Latest known sequence number of the remote node.
    latest_seq: u64,
}

/// Synchronization state of the local node compared to what is known about the
/// remote node.
#[derive(Clone, Default)]
pub struct NodeView {
    sequences: Arc<Mutex<KnownSequences>>,
}

impl NodeView {
    /// Get the baseline sequence (where the local node has already recieved all
    /// available updates).
    pub async fn get_baseline_sequence(&self) -> u64 {
        self.sequences.lock().await.baseline_seq
    }

    /// Update the known synchronization state compared to the remote node.
    pub async fn update(&self, new_sequence: u64) -> bool {
        let mut current = self.sequences.lock().await;
        current.latest_seq = new_sequence;
        if current.baseline_seq + 1 == current.latest_seq {
            // In sync after this update
            current.baseline_seq = new_sequence;
            return true;
        }
        // No longer in sync.. we are missing update(s).
        false
    }
}
