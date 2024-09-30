use crate::bandsim::chain::{MAX_SHARD_BANDWIDTH, MIN_RECEIPT_SIZE};
use crate::bandsim::simulation::builder::SimulationBuilder;
use crate::bandsim::simulation::receipt_sender::{FullSpeedReceiptSender, OneSizeReceiptGenerator};
use crate::bandsim::tests::DEFAULT_TEST_LENGTH;
use crate::bandsim::validation::TestStats;

/// 0 -> 0 - full speed receipts slightly larger than half of max bandwidth
/// 0 -> 1 - full speed small receipts
/// Fairness and utilization should be good.
#[test]
fn medium_vs_small_sender() {
    let simulation_run = SimulationBuilder::new(2)
        .receipt_sender(
            0,
            0,
            FullSpeedReceiptSender(OneSizeReceiptGenerator {
                size: MAX_SHARD_BANDWIDTH / 2 + 100,
            }),
        )
        .receipt_sender(
            0,
            1,
            FullSpeedReceiptSender(OneSizeReceiptGenerator {
                size: MIN_RECEIPT_SIZE,
            }),
        )
        .build()
        .run_for(DEFAULT_TEST_LENGTH);

    let stats = TestStats::new(&simulation_run);
    stats.basic_assert();
    assert!(stats.max_min_ratio.ratio <= 1.10);
    assert!(stats.bandwidth_utilization.utilization > 0.95);
}

/// 0 -> 0 - full speed receipts slightly larger than half of max bandwidth
/// 1 -> 0 - full speed small receipts
/// Fairness and utilization should be good.
#[test]
fn medium_vs_small_receiver() {
    let simulation_run = SimulationBuilder::new(2)
        .receipt_sender(
            0,
            0,
            FullSpeedReceiptSender(OneSizeReceiptGenerator {
                size: MAX_SHARD_BANDWIDTH / 2 + 100,
            }),
        )
        .receipt_sender(
            1,
            0,
            FullSpeedReceiptSender(OneSizeReceiptGenerator {
                size: MIN_RECEIPT_SIZE,
            }),
        )
        .build()
        .run_for(DEFAULT_TEST_LENGTH);

    let stats = TestStats::new(&simulation_run);
    stats.basic_assert();
    assert!(stats.max_min_ratio.ratio <= 1.10);
    assert!(stats.bandwidth_utilization.utilization > 0.95);
}

/// 0 -> 0 - receips slightly larger than half of max bandwidth
/// Bandwidth utilization should be ~50% - scheduler can send only one medium receipt per block height.
#[test]
fn one_medium() {
    let simulation_run = SimulationBuilder::new(2)
        .receipt_sender(
            0,
            0,
            FullSpeedReceiptSender(OneSizeReceiptGenerator {
                size: MAX_SHARD_BANDWIDTH / 2 + 100,
            }),
        )
        .build()
        .run_for(DEFAULT_TEST_LENGTH);

    let stats = TestStats::new(&simulation_run);
    stats.basic_assert();
    assert!(stats.bandwidth_utilization.utilization <= 0.60);
}
