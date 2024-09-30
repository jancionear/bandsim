use std::time::Duration;

use rand::seq::SliceRandom;
use rand::Rng;

use crate::bandsim::chain::{MAX_RECEIPT_SIZE, MAX_SHARD_BANDWIDTH, MIN_RECEIPT_SIZE};
use crate::bandsim::rng::{rng_from_seed, DefaultRng};
use crate::bandsim::simulation::builder::SimulationBuilder;
use crate::bandsim::simulation::receipt_sender::{
    FullSpeedReceiptSender, OneSizeReceiptGenerator, RandomSizeReceiptGenerator, ReceiptSender,
    TypicalReceiptGenerator,
};
use crate::bandsim::validation::TestStats;

use super::DEFAULT_TEST_LENGTH;

#[test]
fn random_size_senders() {
    let simulation_run = SimulationBuilder::new(6)
        .default_sender_factory(|_rng| {
            Box::new(FullSpeedReceiptSender(RandomSizeReceiptGenerator {
                size_range: MIN_RECEIPT_SIZE..=MAX_RECEIPT_SIZE,
            }))
        })
        .build()
        .run_for(DEFAULT_TEST_LENGTH);
    let stats = TestStats::new(&simulation_run);
    stats.basic_assert();
}

fn randomized_test(seed: u64, max_shards: usize) {
    let mut rng = rng_from_seed(seed);
    let num_shards = rng.gen_range(1..=max_shards);
    let simulation_run = SimulationBuilder::new(num_shards)
        .default_sender_factory(random_full_speed_sender)
        .build()
        .run_for(DEFAULT_TEST_LENGTH);
    let stats = TestStats::new(&simulation_run);
    stats.basic_assert();
}

/// A test that runs forever and tries to find a minimal randomized test that fails.
#[test]
fn find_minimal_counterexample() {
    let mut max_shards = 2;
    let increase_shards_duration = Duration::from_secs(60);
    let mut last_increase_time = std::time::Instant::now();
    for seed in 0.. {
        if last_increase_time.elapsed() > increase_shards_duration {
            max_shards += 1;
            last_increase_time = std::time::Instant::now();
        }
        println!("===================== Test with max_shards = {max_shards}, seed = {seed} =====================");
        randomized_test(seed, max_shards);
    }
}

const RANDOMIZED_TEST_MAX_SHARDS: usize = 16;

#[test]
fn randomized_test_0() {
    randomized_test(0, RANDOMIZED_TEST_MAX_SHARDS);
}

#[test]
fn randomized_test_1() {
    randomized_test(1, RANDOMIZED_TEST_MAX_SHARDS);
}

#[test]
fn randomized_test_2() {
    randomized_test(2, RANDOMIZED_TEST_MAX_SHARDS);
}

#[test]
fn randomized_test_3() {
    randomized_test(3, RANDOMIZED_TEST_MAX_SHARDS);
}

#[test]
fn randomized_test_4() {
    randomized_test(4, RANDOMIZED_TEST_MAX_SHARDS);
}

#[test]
fn randomized_test_5() {
    randomized_test(5, RANDOMIZED_TEST_MAX_SHARDS);
}

#[test]
fn randomized_test_6() {
    randomized_test(6, RANDOMIZED_TEST_MAX_SHARDS);
}

#[test]
fn randomized_test_7() {
    randomized_test(7, RANDOMIZED_TEST_MAX_SHARDS);
}

#[test]
fn randomized_test_8() {
    randomized_test(8, RANDOMIZED_TEST_MAX_SHARDS);
}

#[test]
fn randomized_test_9() {
    randomized_test(9, RANDOMIZED_TEST_MAX_SHARDS);
}

/// Reproduces a bug where MAX_SHARD_BANDWIDTH - num_shards * base_bandwidth was smaller than the bandwidth option corresponding to max size receipt.ca
#[test]
fn bug1() {
    randomized_test(13419, 10);
}

pub fn random_full_speed_sender(rng: &mut DefaultRng) -> Box<dyn ReceiptSender> {
    let sender_factories = [
        // Sends only very small receipts
        || -> Box<dyn ReceiptSender> {
            Box::new(FullSpeedReceiptSender(OneSizeReceiptGenerator {
                size: MIN_RECEIPT_SIZE,
            }))
        },
        // Sends only medium receipts
        || -> Box<dyn ReceiptSender> {
            Box::new(FullSpeedReceiptSender(OneSizeReceiptGenerator {
                size: MAX_SHARD_BANDWIDTH / 2 + 100,
            }))
        },
        // Sends only maximum size receipts
        || -> Box<dyn ReceiptSender> {
            Box::new(FullSpeedReceiptSender(OneSizeReceiptGenerator {
                size: MAX_RECEIPT_SIZE,
            }))
        },
        // Sends random receipts
        || -> Box<dyn ReceiptSender> {
            Box::new(FullSpeedReceiptSender(RandomSizeReceiptGenerator {
                size_range: MIN_RECEIPT_SIZE..=MAX_RECEIPT_SIZE,
            }))
        },
        // Sends random small receipts
        || -> Box<dyn ReceiptSender> {
            Box::new(FullSpeedReceiptSender(RandomSizeReceiptGenerator {
                size_range: MIN_RECEIPT_SIZE..=50_000,
            }))
        },
        // Sends random big receipts
        || -> Box<dyn ReceiptSender> {
            Box::new(FullSpeedReceiptSender(RandomSizeReceiptGenerator {
                size_range: 150_000..=MAX_RECEIPT_SIZE,
            }))
        },
        // Sends typical size receipts
        || -> Box<dyn ReceiptSender> {
            Box::new(FullSpeedReceiptSender(TypicalReceiptGenerator::new()))
        },
    ];
    let random_factory = sender_factories.choose(rng).unwrap();
    random_factory()
}
