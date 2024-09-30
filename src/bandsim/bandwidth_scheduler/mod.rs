pub mod distribute_remaining;
use std::collections::{BTreeMap, VecDeque};

use rand::seq::SliceRandom;

use crate::bandsim::bandwidth_request::{BandwidthRequest, BandwidthRequestOptions};
use crate::bandsim::chain::Block;
use crate::bandsim::chain::{ShardLink, ShardUId, MAX_RECEIPT_SIZE, MAX_SHARD_BANDWIDTH};
use crate::bandsim::rng::DefaultRng;

/// Max allowance that a ShardLink can acquire
const MAX_ALLOWANCE: usize = MAX_SHARD_BANDWIDTH;
/// The maximum size of "base" bandwidth that is granted to all shards.
const MAX_BASE_BANDWIDTH: usize = 100_000;

#[derive(Default)]
pub struct BandwidthScheduler {
    /// How much allowance every shard has accumulated. This information is persistend in the shard state on every shard
    /// and must be kept in sync between all shards.
    allowances: BTreeMap<ShardLink, usize>,
    /// How much bandwidth will be granted on every shard.
    granted_bandwdith: BTreeMap<ShardLink, usize>,
    /// How much more the shard is able to send before hitting max sending bandwidth.
    incoming_limits: BTreeMap<ShardUId, usize>,
    /// How much more the shard is able to receive before hitting max receiving bandwidth.
    outgoing_limits: BTreeMap<ShardUId, usize>,
}

impl BandwidthScheduler {
    pub fn new() -> BandwidthScheduler {
        BandwidthScheduler {
            allowances: BTreeMap::new(),
            granted_bandwdith: BTreeMap::new(),
            incoming_limits: BTreeMap::new(),
            outgoing_limits: BTreeMap::new(),
        }
    }

    pub fn run(&mut self, prev_block: &Block, rng: &mut DefaultRng) -> BTreeMap<ShardLink, usize> {
        let all_shards: Vec<ShardUId> = prev_block.chunks.keys().copied().collect();
        if all_shards.is_empty() {
            // No chunks, no bandwidth grants.
            return BTreeMap::new();
        }

        // Reset stuff
        self.granted_bandwdith = BTreeMap::new();
        self.incoming_limits = BTreeMap::new();
        self.outgoing_limits = BTreeMap::new();

        // New height - grant everyone a fair share of allowance
        let base_bandwidth = self.get_base_bandwidth(all_shards.len());
        let allowance_per_height = MAX_SHARD_BANDWIDTH / all_shards.len();
        for from_shard in &all_shards {
            for to_shard in &all_shards {
                let shard_link = ShardLink {
                    from: *from_shard,
                    to: *to_shard,
                };
                self.add_allowance(shard_link, allowance_per_height);
            }
        }

        // First init the incoming and outgoing limits for every shard.
        for (shard_uid, chunk) in &prev_block.chunks {
            self.outgoing_limits.insert(*shard_uid, MAX_SHARD_BANDWIDTH);

            // BandwidthScheduler doesn't allow to send anything to shards where the previous chunk is missing
            let max_incoming_bandwidth = if chunk.is_some() {
                MAX_SHARD_BANDWIDTH
            } else {
                0
            };
            self.incoming_limits
                .insert(*shard_uid, max_incoming_bandwidth);
        }

        // Grant the base bandwidth to everyone
        for from_shard in &all_shards {
            for to_shard in &all_shards {
                // This might fail for shards that have outgoing_limit equal to 0, ignore the error.
                let _ = self.try_grant_additional_bandwidth(
                    ShardLink {
                        from: *from_shard,
                        to: *to_shard,
                    },
                    base_bandwidth,
                );
            }
        }

        // Convert the badwidth requests to a format used in the algorithm.
        // Order the bandwidth requests by the link's allowance, the links with highest allowance have the highest priority.
        let mut requests_by_allowance: BTreeMap<usize, RequestGroup> = BTreeMap::new();
        for (shard_uid, chunk_opt) in prev_block.chunks.iter() {
            if let Some(chunk) = chunk_opt {
                for bandwidth_request in &chunk.bandwidth_requests {
                    let shard_link = ShardLink {
                        from: *shard_uid,
                        to: bandwidth_request.to_shard,
                    };
                    let internal_request = BandwidthIncreaseRequests::from_bandwidth_request(
                        shard_link,
                        bandwidth_request,
                        base_bandwidth,
                    );
                    let allowance = self.get_allowance(shard_link);
                    requests_by_allowance
                        .entry(allowance)
                        .or_insert_with(|| RequestGroup {
                            requests: Vec::new(),
                        })
                        .requests
                        .push(internal_request);
                }
            }
        }

        // Run the main bandwidth scheduler algorithm
        while !requests_by_allowance.is_empty() {
            // Take the group with the most allowance
            let (_allowance, mut request_group) = requests_by_allowance.pop_last().unwrap();
            // Shuffle to keep things fair
            request_group.requests.shuffle(rng);

            // Try to assign next option from the list
            for mut request in request_group.requests {
                let Some(bandwidth_increase) = request.bandwidth_increases.pop_front() else {
                    continue;
                };
                if self
                    .try_grant_additional_bandwidth(request.shard_link, bandwidth_increase)
                    .is_ok()
                {
                    self.decrease_allowance(request.shard_link, bandwidth_increase);
                    let new_allowance = self.get_allowance(request.shard_link);
                    requests_by_allowance
                        .entry(new_allowance)
                        .or_insert(RequestGroup {
                            requests: Vec::new(),
                        })
                        .requests
                        .push(request);
                }
            }
        }

        // Distribute the remaining bandwidth equally between shards.
        // These grants don't decrease allowance.
        let remaining_bandwidth_grants = distribute_remaining::distribute_remaining_bandwidth(
            &self.outgoing_limits,
            &self.incoming_limits,
        );
        for (shard_link, grant) in remaining_bandwidth_grants {
            self.try_grant_additional_bandwidth(shard_link, grant)
                .expect("Distributing remaining bandwidth must succeed");
        }

        std::mem::take(&mut self.granted_bandwdith)
    }

    /// Calculate the base bandwidth that is granted on all links.
    pub fn get_base_bandwidth(&self, num_shards: usize) -> usize {
        let mut base_bandwidth = (MAX_SHARD_BANDWIDTH - MAX_RECEIPT_SIZE) / num_shards;
        if base_bandwidth > MAX_BASE_BANDWIDTH {
            base_bandwidth = MAX_BASE_BANDWIDTH;
        }
        base_bandwidth
    }

    fn try_grant_additional_bandwidth(
        &mut self,
        shard_link: ShardLink,
        bandwidth_increase: usize,
    ) -> Result<(), NotEnoughBandwidthError> {
        let outgoing_limit = self.outgoing_limits.entry(shard_link.from).or_insert(0);
        let incoming_limit = self.incoming_limits.entry(shard_link.to).or_insert(0);

        if bandwidth_increase > *outgoing_limit || bandwidth_increase > *incoming_limit {
            return Err(NotEnoughBandwidthError);
        }

        *self.granted_bandwdith.entry(shard_link).or_insert(0) += bandwidth_increase;
        *outgoing_limit -= bandwidth_increase;
        *incoming_limit -= bandwidth_increase;

        Ok(())
    }

    fn get_allowance(&mut self, shard_link: ShardLink) -> usize {
        self.allowances
            .get(&shard_link)
            .copied()
            .unwrap_or_default()
    }

    fn set_allowance(&mut self, shard_link: ShardLink, amount: usize) {
        self.allowances.insert(shard_link, amount);
    }

    fn add_allowance(&mut self, shard_link: ShardLink, amount: usize) {
        let mut cur_allowance = self.get_allowance(shard_link);
        cur_allowance += amount;
        if cur_allowance > MAX_ALLOWANCE {
            cur_allowance = MAX_ALLOWANCE;
        }

        self.set_allowance(shard_link, cur_allowance);
    }

    fn decrease_allowance(&mut self, shard_link: ShardLink, amount: usize) {
        let cur_allowance = self.get_allowance(shard_link);
        let new_allowance = cur_allowance.saturating_sub(amount);
        self.set_allowance(shard_link, new_allowance);
    }
}

#[derive(Clone, Copy, Debug)]
struct NotEnoughBandwidthError;

// Group of bandwidth requests with the same allowance
struct RequestGroup {
    requests: Vec<BandwidthIncreaseRequests>,
}

/// A BandwidthRequest translated to a format where each "option" is an increase over the previous option instead of an absolute granted value.
#[derive(Debug)]
struct BandwidthIncreaseRequests {
    /// The shard link on which the bandwdith is requested.
    shard_link: ShardLink,
    /// Each of the entries in the queue describes how much additional bandwidth should be granted.
    bandwidth_increases: VecDeque<usize>,
}

impl BandwidthIncreaseRequests {
    fn from_bandwidth_request(
        shard_link: ShardLink,
        bandwidth_request: &BandwidthRequest,
        base_bandwidth: usize,
    ) -> BandwidthIncreaseRequests {
        assert_eq!(shard_link.to, bandwidth_request.to_shard);
        let mut bandwidth_increases = VecDeque::new();
        let mut last_option = base_bandwidth;
        // Get the absolute values of requested bandwidth from bandwidth request.
        let grant_options = BandwidthRequestOptions::from_bitmap(
            &bandwidth_request.grant_options_bitmap,
            base_bandwidth,
            MAX_SHARD_BANDWIDTH,
        );
        for bandwidth_option in grant_options.0 {
            assert!(bandwidth_option > last_option);
            bandwidth_increases.push_back(bandwidth_option - last_option);
            last_option = bandwidth_option;
        }

        BandwidthIncreaseRequests {
            shard_link,
            bandwidth_increases,
        }
    }
}
