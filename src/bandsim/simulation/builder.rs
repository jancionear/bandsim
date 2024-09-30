use std::collections::BTreeMap;

use crate::bandsim::chain::{ShardLink, ShardUId};
use crate::bandsim::rng::{rng_from_seed, DefaultRng};

use super::receipt_sender::{NoReceiptSender, ReceiptSender};
use super::{MissingChunkGenerator, Simulation};

pub struct SimulationBuilder {
    shards: Vec<ShardUId>,
    receipt_senders: BTreeMap<ShardLink, Box<dyn ReceiptSender>>,
    random_seed: u64,
    default_sender_factory: Option<ReceiptSenderFactory>,
    missing_chunk_generator: Option<MissingChunkGenerator>,
    missing_block_probability: f64,
}

/// A function used to create new receipt senders
type ReceiptSenderFactory = Box<dyn FnMut(&mut DefaultRng) -> Box<dyn ReceiptSender>>;

impl SimulationBuilder {
    /// Create a new simulation with this many shards
    pub fn new(num_shards: usize) -> SimulationBuilder {
        let shards = (0..num_shards).map(ShardUId::new).collect();
        SimulationBuilder {
            shards,
            receipt_senders: BTreeMap::new(),
            random_seed: 0,
            default_sender_factory: None,
            missing_block_probability: 0.0,
            missing_chunk_generator: None,
        }
    }

    /// Set a receipt sender between two shards.
    /// For shard links that didn't have the sender set, builder will use the default sender factory to create one.
    pub fn receipt_sender(
        mut self,
        from_shard: usize,
        to_shard: usize,
        sender: impl ReceiptSender + 'static,
    ) -> Self {
        let shard_link = ShardLink {
            from: ShardUId::new(from_shard),
            to: ShardUId::new(to_shard),
        };

        if self.receipt_senders.contains_key(&shard_link) {
            panic!("There's already a receipt sender for {:?}", shard_link);
        }

        self.receipt_senders.insert(shard_link, Box::new(sender));
        self
    }

    /// Random seed used by the simulation.
    /// Default sender factory doesn't use this seed.
    pub fn random_seed(mut self, seed: u64) -> Self {
        self.random_seed = seed;
        self
    }

    /// Specify a function that will be used to create receipt senders for shard links that didn't have a sender set explicitly.
    pub fn default_sender_factory(
        mut self,
        f: impl FnMut(&mut DefaultRng) -> Box<dyn ReceiptSender> + 'static,
    ) -> Self {
        if self.default_sender_factory.is_some() {
            panic!("Default sender factory is already set!");
        }
        self.default_sender_factory = Some(Box::new(f));
        self
    }

    pub fn missing_block_probability(mut self, p: f64) -> Self {
        self.missing_block_probability = p;
        self
    }

    pub fn missing_chunk_generator(
        mut self,
        generator: impl FnMut(usize, ShardUId, &mut DefaultRng) -> bool + 'static,
    ) -> Self {
        self.missing_chunk_generator = Some(Box::new(generator));
        self
    }

    /// Build the simulation
    pub fn build(mut self) -> Simulation {
        if let Some(mut sender_factory) = self.default_sender_factory.take() {
            let mut create_senders_rng = rng_from_seed(self.random_seed);
            for from_shard in &self.shards {
                for to_shard in &self.shards {
                    let shard_link = ShardLink {
                        from: *from_shard,
                        to: *to_shard,
                    };
                    self.receipt_senders
                        .entry(shard_link)
                        .or_insert_with(|| sender_factory(&mut create_senders_rng));
                }
            }
        }

        Simulation::new(
            self.shards,
            self.receipt_senders,
            self.random_seed,
            self.missing_block_probability,
            self.missing_chunk_generator,
        )
    }
}

#[test]
fn builder_doesnt_crash() {
    let _simulation = SimulationBuilder::new(2)
        .random_seed(0)
        .receipt_sender(0, 1, NoReceiptSender)
        .default_sender_factory(|_rng| Box::new(NoReceiptSender))
        .missing_block_probability(0.01)
        .missing_chunk_generator(|_, _, _| false)
        .build();
}
