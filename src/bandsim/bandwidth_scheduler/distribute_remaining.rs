use std::collections::BTreeMap;

use crate::bandsim::chain::{ShardLink, ShardUId};

/// Magic algorithm which distributes the remaining bandwidth in a fair way (∩ ͡° ͜ʖ ͡°)⊃━☆ﾟ. * ･ ｡ﾟ,
/// The arguments describe how much spare bandwidth there is on the left (sending) shards and right (receiving) shards.
/// The function grants some additional bandwidth on all the links to make use of the leftover bandwidth.
pub fn distribute_remaining_bandwidth(
    left: &BTreeMap<ShardUId, usize>,
    right: &BTreeMap<ShardUId, usize>,
) -> BTreeMap<ShardLink, usize> {
    let left_sum: usize = left.iter().map(|(_shard, bandwidth)| bandwidth).sum();
    let right_sum: usize = right.iter().map(|(_shard, bandwidth)| bandwidth).sum();

    if right_sum < left_sum {
        let flipped_res = distribute_remaining_bandwidth(right, left);
        let res = flipped_res
            .into_iter()
            .map(|(shard_link, bandwidth)| {
                (
                    ShardLink {
                        from: shard_link.to,
                        to: shard_link.from,
                    },
                    bandwidth,
                )
            })
            .collect();
        return res;
    }

    let mut left_by_bandwidth: Vec<(usize, ShardUId)> = left
        .iter()
        .map(|(shard, bandwidth)| (*bandwidth, *shard))
        .collect();
    left_by_bandwidth.sort();

    let mut right_by_bandwidth: Vec<(usize, ShardUId)> = right
        .iter()
        .map(|(shard, bandwidth)| (*bandwidth, *shard))
        .collect();
    right_by_bandwidth.sort();

    let mut bandwidth_grants: BTreeMap<ShardLink, usize> = BTreeMap::new();

    let mut left_num = left_by_bandwidth.len();
    for (mut left_bandwidth, left_shard) in left_by_bandwidth {
        let mut right_num = right_by_bandwidth.len();
        for (right_bandwidth, right_shard) in right_by_bandwidth.iter_mut() {
            let left_max = left_bandwidth / right_num + left_bandwidth % right_num;
            let right_max = *right_bandwidth / left_num + *right_bandwidth % left_num;
            let granted_bandwidth = std::cmp::min(left_max, right_max);

            bandwidth_grants.insert(
                ShardLink {
                    from: left_shard,
                    to: *right_shard,
                },
                granted_bandwidth,
            );

            *right_bandwidth -= granted_bandwidth;
            left_bandwidth -= granted_bandwidth;

            right_num -= 1;
        }

        left_num -= 1;
        assert_eq!(left_bandwidth, 0);
    }

    bandwidth_grants
}
