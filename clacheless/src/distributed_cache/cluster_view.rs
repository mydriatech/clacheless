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

//! Local view of the other nodes.

mod local_sequence;
mod node_view;

use self::local_sequence::LocalSequence;
use self::node_view::NodeView;
use crossbeam_skiplist::SkipMap;
use crossbeam_skiplist::map::Entry;
use std::collections::HashMap;
use std::sync::Arc;

/// Maintains a view of the cluster state from the local nodes perspective.
pub struct ClusterStateView {
    local_sequence: LocalSequence,
    other_nodes_update_seqs: SkipMap<u64, NodeView>,
}

impl ClusterStateView {
    /// Return a new instance.
    pub fn new(local_node_id: u64) -> Arc<Self> {
        Arc::new(Self {
            local_sequence: LocalSequence::new(local_node_id),
            other_nodes_update_seqs: SkipMap::default(),
        })
    }

    /// Return the next unique sequence number for locally recieved cache
    /// writes.
    pub fn next_local_update_seq(&self) -> u64 {
        self.local_sequence.generate_next()
    }

    /// Get a map of `node_id` and the sequence baseline for each known node.
    pub async fn as_map(&self) -> HashMap<u64, u64> {
        let mut ret = HashMap::with_capacity(self.other_nodes_update_seqs.len() + 1);
        if self.local_sequence.has_been_pulled() {
            // Add local state
            ret.insert(self.local_sequence.node_id(), self.local_sequence.current());
        }
        for entry in self.other_nodes_update_seqs.iter() {
            let node_id = *entry.key();
            let baseline = entry.value().get_baseline_sequence().await;
            ret.insert(node_id, baseline);
        }
        ret
    }

    /// Compare recieved view with local view and return a map of nodes that
    /// require a state transfer and each node's current local baseline.
    pub async fn get_out_of_sync_node_id_and_baselines(
        &self,
        view: HashMap<u64, u64>,
    ) -> HashMap<u64, u64> {
        let mut ret = HashMap::new();
        // Ignore if we know more than the other node, just check if that node
        // knowns more than we do.
        for (node_id, baseline_seq) in view {
            if node_id == self.local_sequence.node_id() {
                // Don't compare with local state where this instance is authoritive.
                continue;
            }
            if let Some(node_view) = self
                .other_nodes_update_seqs
                .get(&node_id)
                .as_ref()
                .map(Entry::value)
                .cloned()
            {
                let other_baseline = node_view.get_baseline_sequence().await;
                if other_baseline < baseline_seq {
                    ret.insert(node_id, other_baseline);
                }
            } else {
                ret.insert(node_id, 0);
            }
        }
        ret
    }

    /// Returns `false` if we are missing updates for the `node_id`.
    pub async fn on_recieved_cache_entry_from_other(&self, node_id: u64, update_seq: u64) -> bool {
        let entry = self
            .other_nodes_update_seqs
            .get_or_insert_with(node_id, NodeView::default);
        entry.value().update(update_seq).await
    }
}
