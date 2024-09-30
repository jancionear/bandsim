use std::collections::BTreeMap;
use std::time::Duration;

use rand::seq::SliceRandom;
use rand::Rng;

use crate::bandsim::bandwidth_scheduler::distribute_remaining::distribute_remaining_bandwidth;
use crate::bandsim::chain::{ShardLink, ShardUId, MAX_SHARD_BANDWIDTH};
use crate::bandsim::rng::{rng_from_seed, DefaultRng};

fn generate_shards(rng: &mut DefaultRng) -> Vec<ShardUId> {
    let num_shards: usize = rng.gen_range(1..10);
    (0..num_shards)
        .map(|shard_id| ShardUId {
            version: 0,
            shard_id: shard_id as u32,
        })
        .collect()
}

fn generate_shard_bandwidth(rng: &mut DefaultRng) -> usize {
    rng.gen_range(0..=MAX_SHARD_BANDWIDTH)
}

/// Generate limits (left or right argument to distribute_remaining_bandwdith()) that have these shards and this much total badwidth available.
/// Having total_bandwidth allows to create test cases where both sides have the same amount of available bandwidth without having the same per-shard limits.
fn generate_limits(
    shards: &[ShardUId],
    total_bandwidth: usize,
    rng: &mut DefaultRng,
) -> BTreeMap<ShardUId, usize> {
    // First generate limits where every shard has an equal limit.
    let mut limits: BTreeMap<ShardUId, usize> = shards
        .iter()
        .map(|shard_uid| (*shard_uid, total_bandwidth / shards.len()))
        .collect();

    let remaining = total_bandwidth % shards.len();
    *limits.get_mut(&shards[0]).unwrap() += remaining;

    let get_sum =
        |l: &BTreeMap<ShardUId, usize>| -> usize { l.iter().map(|(_shard, limit)| limit).sum() };

    assert_eq!(get_sum(&limits), total_bandwidth);

    // Then move a random amount between random pairs of shards to make the per-shard limits random.
    for _ in 0..(shards.len() * 10) {
        let shard1 = shards.choose(rng).unwrap();
        let shard2 = shards.choose(rng).unwrap();

        if shard1 == shard2 {
            continue;
        }

        let max_moved = std::cmp::min(
            *limits.get(shard1).unwrap(),
            MAX_SHARD_BANDWIDTH - *limits.get(shard2).unwrap(),
        );
        if max_moved > 0 {
            let moved = rng.gen_range(0..max_moved);
            *limits.get_mut(shard1).unwrap() -= moved;
            *limits.get_mut(shard2).unwrap() += moved;
        }
    }

    assert_eq!(get_sum(&limits), total_bandwidth);
    limits
}

#[derive(Debug)]
struct TestCase {
    left: BTreeMap<ShardUId, usize>,
    right: BTreeMap<ShardUId, usize>,
    workload_type: &'static str,
}

#[derive(Debug)]
struct TestCaseError {
    bandwidth_grants: BTreeMap<ShardLink, usize>,
    cause: TestCaseErrorCause,
}

#[allow(unused)]
#[derive(Debug)]
enum TestCaseErrorCause {
    GrantsSumMismatch {
        left_sum: usize,
        right_sum: usize,
        grants_sum: usize,
    },
    LeftLimitNotRespected {
        left_shard_id: ShardUId,
        left_limit: usize,
        grant_link: ShardLink,
        grant_size: usize,
    },
    RightLimitNotRespected {
        right_shard_id: ShardUId,
        right_limit: usize,
        grant_link: ShardLink,
        grant_size: usize,
    },
}

impl TestCase {
    fn generate_new(rng: &mut DefaultRng) -> TestCase {
        let shards = generate_shards(rng);

        let workload_types = [
            "random",
            "equal_total",
            "slightly_different_total",
            "identical",
        ];
        let workload_type = *workload_types.choose(rng).unwrap();

        fn get_total_bandwidth(shards: &[ShardUId], rng: &mut DefaultRng) -> usize {
            generate_shard_bandwidth(rng) * shards.len()
        }

        let (left, right) = match workload_type {
            "random" => {
                let b1 = get_total_bandwidth(&shards, rng);
                let b2 = get_total_bandwidth(&shards, rng);
                (
                    generate_limits(&shards, b1, rng),
                    generate_limits(&shards, b2, rng),
                )
            }
            "equal_total" => {
                let total_bandwidth: usize = get_total_bandwidth(&shards, rng);

                (
                    generate_limits(&shards, total_bandwidth, rng),
                    generate_limits(&shards, total_bandwidth, rng),
                )
            }
            "slightly_different_total" => {
                let total_bandwidth: usize = get_total_bandwidth(&shards, rng).saturating_sub(100); // Make sure it's not MAX*num_shards

                (
                    generate_limits(&shards, total_bandwidth, rng),
                    generate_limits(&shards, total_bandwidth + 10, rng), // Add a bit to the second limits
                )
            }
            "identical" => {
                let limits = generate_limits(&shards, get_total_bandwidth(&shards, rng), rng);
                (limits.clone(), limits)
            }
            other => panic!("Got {}, thats unexpected", other),
        };

        TestCase {
            left,
            right,
            workload_type,
        }
    }

    fn run_test(&self) -> Result<(), TestCaseError> {
        let bandwidth_grants = distribute_remaining_bandwidth(&self.left, &self.right);

        let left_sum: usize = self.left.values().sum();
        let right_sum: usize = self.right.values().sum();
        let expected_grants = std::cmp::min(left_sum, right_sum);

        let grants_sum: usize = bandwidth_grants.values().sum();

        // Make sure that all of the available bandwidth is used.
        if grants_sum != expected_grants {
            return Err(TestCaseError {
                bandwidth_grants,
                cause: TestCaseErrorCause::GrantsSumMismatch {
                    left_sum,
                    right_sum,
                    grants_sum,
                },
            });
        }

        // Make sure that the limits are respected
        for (link, grant) in &bandwidth_grants {
            let from_limit = self.left.get(&link.from).unwrap();
            if grant > from_limit {
                return Err(TestCaseError {
                    bandwidth_grants: bandwidth_grants.clone(),
                    cause: TestCaseErrorCause::LeftLimitNotRespected {
                        left_shard_id: link.from,
                        left_limit: *from_limit,
                        grant_link: *link,
                        grant_size: *grant,
                    },
                });
            }

            let to_limit = self.right.get(&link.to).unwrap();
            if grant > to_limit {
                return Err(TestCaseError {
                    bandwidth_grants: bandwidth_grants.clone(),
                    cause: TestCaseErrorCause::RightLimitNotRespected {
                        right_shard_id: link.to,
                        right_limit: *to_limit,
                        grant_link: *link,
                        grant_size: *grant,
                    },
                });
            }
        }

        Ok(())
    }

    fn run(&self) {
        let Err(err) = self.run_test() else { return };

        println!("ERROR!!!!");
        println!("Num shards: {}", self.left.len());
        println!("test case:");
        println!("TestCase {{");
        println!("    left: limits_from_data(&[");
        for (shard, bandwidth) in &self.left {
            println!("        ({}, {}),", shard.shard_id, bandwidth);
        }
        println!("    ]),");
        println!("    right: limits_from_data(&[");
        for (shard, bandwidth) in &self.right {
            println!("        ({}, {}),", shard.shard_id, bandwidth);
        }
        println!("    ]),");
        println!("    workload_type: \"{}\",", self.workload_type);
        println!("}}");
        println!("BANDWIDTH_GRANTS:");
        for (link, grant) in err.bandwidth_grants {
            println!(
                "({} -> {}) - {}",
                link.from.shard_id, link.to.shard_id, grant
            );
        }
        println!("ERROR CAUSE:");
        println!("{:#?}", err.cause);

        panic!("Test case failed!");
    }
}

#[test]
fn test_distributing_remaining_random() {
    let random_tests_num = 100_000;
    let mut rng = rng_from_seed(0);

    let mut last_report_time = std::time::Instant::now();
    for test_num in 0_u64..random_tests_num {
        let test_case = TestCase::generate_new(&mut rng);
        test_case.run();

        if last_report_time.elapsed() > Duration::from_secs(1) {
            println!("{} tests OK", pretty_number(test_num));
            last_report_time = std::time::Instant::now();
        }
    }
}

fn pretty_number(num: u64) -> String {
    let num_str = num.to_string();

    if num_str.len() <= 3 {
        return num_str;
    }

    let mut result = String::new();
    let remainder = num_str.len() % 3;
    for (i, c) in num_str.chars().enumerate() {
        result.push(c);
        if (i + 1) % 3 == remainder {
            result.push('_');
        }
    }
    result.pop();
    result
}

#[test]
fn test_pretty_number() {
    assert_eq!(&pretty_number(0), "0");
    assert_eq!(&pretty_number(1), "1");
    assert_eq!(&pretty_number(12), "12");
    assert_eq!(&pretty_number(123), "123");
    assert_eq!(&pretty_number(1234), "1_234");
    assert_eq!(&pretty_number(12345), "12_345");
    assert_eq!(&pretty_number(123456), "123_456");
    assert_eq!(&pretty_number(1234567), "1_234_567");
    assert_eq!(&pretty_number(12345678), "12_345_678");
    assert_eq!(&pretty_number(123456789), "123_456_789");
}

fn limits_from_data(data: &[(usize, usize)]) -> BTreeMap<ShardUId, usize> {
    data.iter()
        .map(|(shard_id, limit)| {
            (
                ShardUId {
                    shard_id: *shard_id as u32,
                    version: 0,
                },
                *limit,
            )
        })
        .collect()
}

#[test]
fn test_three_100() {
    let test_case = TestCase {
        left: limits_from_data(&[(0, 100), (1, 100), (2, 100)]),
        right: limits_from_data(&[(0, 100), (1, 100), (2, 100)]),
        workload_type: "custom1",
    };
    test_case.run();
}
