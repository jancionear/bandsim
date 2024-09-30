use crate::bandsim::chain::{MAX_RECEIPT_SIZE, MIN_RECEIPT_SIZE};
use crate::bandsim::simulation::builder::SimulationBuilder;
use crate::bandsim::simulation::receipt_sender::{FullSpeedReceiptSender, OneSizeReceiptGenerator};
use crate::bandsim::validation::TestStats;

use super::DEFAULT_TEST_LENGTH;

fn big_sender() -> FullSpeedReceiptSender<OneSizeReceiptGenerator> {
    FullSpeedReceiptSender(OneSizeReceiptGenerator {
        size: MAX_RECEIPT_SIZE,
    })
}

fn small_sender() -> FullSpeedReceiptSender<OneSizeReceiptGenerator> {
    FullSpeedReceiptSender(OneSizeReceiptGenerator {
        size: MIN_RECEIPT_SIZE,
    })
}

/// 0 -> 0 - full speed big receipts
/// 0 -> 1 - full speed small receipts
/// /// Fairness and utilization should be good.
#[test]
fn big_vs_small_sender() {
    let simulation_run = SimulationBuilder::new(2)
        .receipt_sender(0, 0, big_sender())
        .receipt_sender(0, 1, small_sender())
        .build()
        .run_for(DEFAULT_TEST_LENGTH);

    let stats = TestStats::new(&simulation_run);
    stats.basic_assert();
    assert!(stats.max_min_ratio.ratio <= 1.25);
    assert!(stats.bandwidth_utilization.utilization > 0.90);
}

/// 0 -> 0 - full speed big receipts
/// 1 -> 0 - full speed small receipts
/// Fairness and utilization should be good.
#[test]
fn big_vs_small_receiver() {
    let simulation_run = SimulationBuilder::new(2)
        .receipt_sender(0, 0, big_sender())
        .receipt_sender(1, 0, small_sender())
        .build()
        .run_for(DEFAULT_TEST_LENGTH);

    let stats = TestStats::new(&simulation_run);
    stats.basic_assert();
    assert!(stats.max_min_ratio.ratio <= 1.25);
    assert!(stats.bandwidth_utilization.utilization > 0.90);
}

/// 0 -> 0 - full speed big receipts
/// 0 -> 1 - full speed small receipts
/// 0 -> 2 - full speed small receipts
/// 0 -> 3 - full speed small receipts
/// 0 -> 4 - full speed small receipts
/// 0 -> 5 - full speed small receipts
/// Fairness and utilization should be good.
#[test]
fn big_vs_many_small_sender() {
    let simulation_run = SimulationBuilder::new(5)
        .receipt_sender(0, 0, big_sender())
        .receipt_sender(0, 1, small_sender())
        .receipt_sender(0, 2, small_sender())
        .receipt_sender(0, 3, small_sender())
        .receipt_sender(0, 4, small_sender())
        .build()
        .run_for(DEFAULT_TEST_LENGTH);

    let stats = TestStats::new(&simulation_run);
    stats.basic_assert();
    assert!(stats.max_min_ratio.ratio <= 1.15);
    assert!(stats.bandwidth_utilization.utilization > 0.90);
}

/// 0 -> 0 - full speed big receipts
/// 1 -> 0 - full speed small receipts
/// 2 -> 0 - full speed small receipts
/// 3 -> 0 - full speed small receipts
/// 4 -> 0 - full speed small receipts
/// Fairness and utilization should be good.
#[test]
fn big_vs_many_small_receiver() {
    let simulation_run = SimulationBuilder::new(5)
        .receipt_sender(0, 0, big_sender())
        .receipt_sender(1, 0, small_sender())
        .receipt_sender(2, 0, small_sender())
        .receipt_sender(3, 0, small_sender())
        .receipt_sender(4, 0, small_sender())
        .build()
        .run_for(DEFAULT_TEST_LENGTH);

    let stats = TestStats::new(&simulation_run);
    stats.basic_assert();
    assert!(stats.max_min_ratio.ratio <= 1.15);
    assert!(stats.bandwidth_utilization.utilization > 0.90);
}
