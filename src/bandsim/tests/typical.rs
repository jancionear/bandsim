use rand::Rng;

use crate::bandsim::simulation::builder::SimulationBuilder;
use crate::bandsim::simulation::receipt_sender::{FullSpeedReceiptSender, TypicalReceiptGenerator};
use crate::bandsim::validation::TestStats;

use super::DEFAULT_TEST_LENGTH;

/// Typical case - typical receipts, a bit of missing chunks and blocks.
#[test]
fn typical_test() {
    let simulation_run = SimulationBuilder::new(6)
        .default_sender_factory(|_rng| {
            Box::new(FullSpeedReceiptSender(TypicalReceiptGenerator::new()))
        })
        .missing_block_probability(0.05)
        .missing_chunk_generator(|_, _, rng| rng.gen_bool(0.05))
        .build()
        .run_for(DEFAULT_TEST_LENGTH);
    let stats = TestStats::new(&simulation_run);
    stats.basic_assert();
    assert!(stats.max_min_ratio.ratio <= 1.20);
    assert!(stats.bandwidth_utilization.utilization > 0.75);
}
