use rand::Rng;

use crate::bandsim::chain::ShardUId;
use crate::bandsim::simulation::builder::SimulationBuilder;
use crate::bandsim::simulation::receipt_sender::{FullSpeedReceiptSender, TypicalReceiptGenerator};
use crate::bandsim::validation::TestStats;

use super::DEFAULT_TEST_LENGTH;

/// Run a simulation where 10% of chunks are missing.
/// TypicalSender sends mostly small receipts.
#[test]
fn ten_percent_missing_chunks() {
    let simulation_run = SimulationBuilder::new(6)
        .default_sender_factory(|_rng| {
            Box::new(FullSpeedReceiptSender(TypicalReceiptGenerator::new()))
        })
        .missing_chunk_generator(|_height, _id, rng| rng.gen_bool(0.1))
        .build()
        .run_for(DEFAULT_TEST_LENGTH);
    let stats = TestStats::new(&simulation_run);
    stats.basic_assert();
    assert!(stats.missing_chunks_ratio > 0.08);
    assert!(stats.missing_chunks_ratio < 0.12);
}

/// Run a simulation where 10% of chunks on shard 0 are missing.
/// TypicalSender sends mostly small receipts.
#[test]
fn ten_percent_missing_chunks_on_shard0() {
    let simulation_run = SimulationBuilder::new(6)
        .default_sender_factory(|_rng| {
            Box::new(FullSpeedReceiptSender(TypicalReceiptGenerator::new()))
        })
        .missing_chunk_generator(|_height, id, rng| {
            if id != ShardUId::new(0) {
                return false;
            }
            rng.gen_bool(0.1)
        })
        .build()
        .run_for(DEFAULT_TEST_LENGTH);
    let stats = TestStats::new(&simulation_run);
    stats.basic_assert();
    assert!(stats.missing_chunks_ratio > 0.01);
    assert!(stats.missing_chunks_ratio < 0.02);
}

/// 10 chunks aren't missing, then 10 chunks are missing, then 10 are missing, etc on shard 0
/// TypicalSender sends mostly small receipts.
#[test]
fn ten_missing_ten_not_missing_on_shard0() {
    let simulation_run = SimulationBuilder::new(6)
        .default_sender_factory(|_rng| {
            Box::new(FullSpeedReceiptSender(TypicalReceiptGenerator::new()))
        })
        .missing_chunk_generator(|height, id, _rng| {
            if id != ShardUId::new(0) {
                return false;
            }
            (height / 10) % 2 == 1
        })
        .build()
        .run_for(DEFAULT_TEST_LENGTH);
    let stats = TestStats::new(&simulation_run);

    // Standard fairness check fails because shard0 processes half the receipts that other shards do. This is expected.
    assert!(stats.max_min_ratio.ratio <= 3.0);
    assert!(stats.bandwidth_utilization.utilization >= 0.6);

    assert!(stats.missing_chunks_ratio > 0.05);
    assert!(stats.missing_chunks_ratio < 0.10);
}
