use std::collections::BTreeMap;

use outgoing_queue::OutgoingQueue;
use rand::Rng;
use receipt_sender::ReceiptSender;

use crate::bandsim::bandwidth_scheduler::BandwidthScheduler;
use crate::bandsim::chain::{Block, Chunk, ShardLink, ShardUId};
use crate::bandsim::rng::{rng_from_seed, DefaultRng};
use crate::bandsim::validation::{validate_block, validate_grants};

pub mod builder;
pub mod outgoing_queue;
pub mod receipt_sender;

/// Simulates the blockchain.
/// Generates blocks and chunks, uses bandwidth scheduler to schedule bandwidth, sends receipts between shards.
pub struct Simulation {
    pub shards: BTreeMap<ShardUId, Shard>,
    pub blocks: Vec<Option<Block>>,
    pub rng: DefaultRng,
    pub missing_block_probability: f64,
    pub missing_chunk_generator: MissingChunkGenerator,
}

/// A function which takes the block heightand shard id and decides whether the chunk should be missing.
pub type MissingChunkGenerator = Box<dyn FnMut(usize, ShardUId, &mut DefaultRng) -> bool>;

/// This structs exists to ensure that the simulation actually runs before performing checks.
/// Validatoin checks take a `SimulationRun` which ensures that the simulation was run
/// before performing the checks.
pub struct SimulationRun {
    pub simulation: Simulation,
}

impl Simulation {
    /// Create a new simulation.
    /// It's usually more convenient to use `SimulationBuilder`.
    pub fn new(
        shard_ids: Vec<ShardUId>,
        mut receipt_senders: BTreeMap<ShardLink, Box<dyn ReceiptSender>>,
        random_seed: u64,
        missing_block_probability: f64,
        missing_generator: Option<MissingChunkGenerator>,
    ) -> Simulation {
        let rng = rng_from_seed(random_seed);

        let mut shards = BTreeMap::new();
        for shard_id in &shard_ids {
            let mut shard_senders = BTreeMap::new();
            for to_shard in &shard_ids {
                if let Some(link_sender) = receipt_senders.remove(&ShardLink {
                    from: *shard_id,
                    to: *to_shard,
                }) {
                    shard_senders.insert(*to_shard, link_sender);
                }
            }
            shards.insert(*shard_id, Shard::new(*shard_id, &shard_ids, shard_senders));
        }

        let missing_chunk_generator =
            missing_generator.unwrap_or_else(|| Box::new(|_height, _shard_id, _rng| false));

        let res = Simulation {
            shards,
            blocks: vec![Some(Self::make_genesis_block(&shard_ids))],
            rng,
            missing_block_probability,
            missing_chunk_generator,
        };
        // Automatically information about the simulation for every created simulation.
        // Less repetition in tests.
        res.print_info();
        res
    }

    fn make_genesis_block(shard_ids: &[ShardUId]) -> Block {
        let mut genesis_block = Block {
            height: 0,
            chunks: BTreeMap::new(),
        };
        for shard_id in shard_ids {
            let genesis_chunk = Chunk {
                prev_incoming_receipts_size: 0,
                prev_outgoing_receipts_size: BTreeMap::new(),
                bandwidth_requests: Vec::new(),
            };
            genesis_block.chunks.insert(*shard_id, Some(genesis_chunk));
        }
        validate_block(&genesis_block, &[]);
        genesis_block
    }

    /// Move the simulation one block forward
    fn step(&mut self) {
        let is_block_missing = self.rng.gen_bool(self.missing_block_probability);
        if is_block_missing {
            self.blocks.push(None);
            return;
        }

        let mut new_block = Block {
            height: self.blocks.len(),
            chunks: BTreeMap::new(),
        };

        for (shard_uid, shard) in self.shards.iter_mut() {
            shard.next_height(&self.blocks);

            let is_chunk_missing =
                (self.missing_chunk_generator)(new_block.height, *shard_uid, &mut self.rng);
            if is_chunk_missing {
                new_block.chunks.insert(*shard_uid, None);
            } else {
                let new_chunk = shard.apply_and_produce_chunk(&self.blocks, &mut self.rng);
                new_block.chunks.insert(*shard_uid, Some(new_chunk));
            }
        }

        validate_block(&new_block, &self.blocks);

        self.blocks.push(Some(new_block));
    }

    /// Run the simulation for this many blocks.
    pub fn run_for(mut self, steps: usize) -> SimulationRun {
        for _ in 0..steps {
            self.step();
        }
        SimulationRun { simulation: self }
    }

    pub fn print_info(&self) {
        println!("Simulation");
        println!("shards num: {}", self.shards.len());
        println!("Receipt Senders:");
        for (sid, shard) in &self.shards {
            for (to_shard, sender) in &shard.receipt_senders {
                let shard_link = ShardLink {
                    from: *sid,
                    to: *to_shard,
                };
                println!("{:?}: {:?}", shard_link, sender);
            }
        }
    }
}

pub struct Shard {
    pub id: ShardUId,
    pub bandwidth_scheduler: BandwidthScheduler,
    pub latest_grants: BTreeMap<ShardLink, usize>,
    pub outgoing_queues: BTreeMap<ShardUId, OutgoingQueue>,
    pub receipt_senders: BTreeMap<ShardUId, Box<dyn ReceiptSender>>,
}

fn last_non_missing_block(past_blocks: &[Option<Block>]) -> &Block {
    for block_opt in past_blocks.iter().rev() {
        if let Some(block) = block_opt {
            return block;
        }
    }
    panic!("All blocks are missing!");
}

impl Shard {
    fn new(
        id: ShardUId,
        shard_ids: &[ShardUId],
        mut receipt_senders_in: BTreeMap<ShardUId, Box<dyn ReceiptSender>>,
    ) -> Shard {
        let mut outgoing_queues = BTreeMap::new();
        let mut receipt_senders = BTreeMap::new();
        for shard_id in shard_ids {
            outgoing_queues.insert(*shard_id, OutgoingQueue::new(*shard_id));

            if let Some(sender) = receipt_senders_in.remove(shard_id) {
                receipt_senders.insert(*shard_id, sender);
            }
        }
        assert!(receipt_senders_in.is_empty());

        Shard {
            id,
            bandwidth_scheduler: BandwidthScheduler::new(),
            latest_grants: BTreeMap::new(),
            outgoing_queues,
            receipt_senders,
        }
    }

    /// Update the local state on the next height.
    /// This happens on every height with a non-missing block even when the chunk on this shard is missing.
    /// BandwidthScheduler has to be run on every height to keep its state on all shards in sync.
    fn next_height(&mut self, past_blocks: &[Option<Block>]) {
        let last_block = last_non_missing_block(past_blocks);
        // In reality the rng used by BandwidthScheduler would be derived from the Block's hash.
        let mut rng = rng_from_seed(last_block.height as u64);
        self.latest_grants = self.bandwidth_scheduler.run(last_block, &mut rng);
        validate_grants(&self.latest_grants);
    }

    /// Applies the last chunk on this shard and produces a new one
    fn apply_and_produce_chunk(
        &mut self,
        past_blocks: &[Option<Block>],
        rng: &mut DefaultRng,
    ) -> Chunk {
        // Gather incoming receipts from previous heights
        let mut incoming_receipts_size = 0;
        for block_opt in past_blocks.iter().rev() {
            let Some(block) = block_opt else {
                continue;
            };

            let mut this_shard_non_missing = false;
            for (shard_uid, chunk_opt) in &block.chunks {
                if *shard_uid == self.id && chunk_opt.is_some() {
                    this_shard_non_missing = true;
                }

                let Some(chunk) = chunk_opt else {
                    continue;
                };
                let cur_incoming_receipts_size = chunk
                    .prev_outgoing_receipts_size
                    .get(&self.id)
                    .unwrap_or(&0);
                incoming_receipts_size += cur_incoming_receipts_size;
            }
            if this_shard_non_missing {
                break;
            }
        }

        // Send outgoing receipts using the granted bandwidth
        let mut outgoing_receipt_sizes: BTreeMap<ShardUId, usize> = BTreeMap::new();
        for (to_shard, outgoing_queue) in self.outgoing_queues.iter_mut() {
            let shard_link = ShardLink {
                from: self.id,
                to: *to_shard,
            };
            let mut link_grant = self.latest_grants.get(&shard_link).copied().unwrap_or(0);
            let mut link_outgoing_receipts_size = 0;
            while !outgoing_queue.is_empty()
                && link_grant >= outgoing_queue.first_receipt_size().unwrap()
            {
                let receipt = outgoing_queue.pop().unwrap();
                link_outgoing_receipts_size += receipt.size;
                link_grant -= receipt.size;
            }
            outgoing_receipt_sizes.insert(*to_shard, link_outgoing_receipts_size);
        }

        // Generate new receipts
        for (to_shard, receipt_sender) in self.receipt_senders.iter_mut() {
            let outgoing_queue = self.outgoing_queues.get_mut(to_shard).unwrap();

            receipt_sender.send_receipts(outgoing_queue, rng);
        }

        // Generate bandwidth requests
        let last_block = last_non_missing_block(past_blocks);
        let num_shards = last_block.chunks.len();
        let base_bandwidth = self.bandwidth_scheduler.get_base_bandwidth(num_shards);
        let mut bandwidth_requests = Vec::new();
        for outgoing_queue in self.outgoing_queues.values_mut() {
            if let Some(bandwidth_request) = outgoing_queue.make_bandwidth_request(base_bandwidth) {
                bandwidth_requests.push(bandwidth_request);
            }
        }

        Chunk {
            prev_incoming_receipts_size: incoming_receipts_size,
            prev_outgoing_receipts_size: outgoing_receipt_sizes,
            bandwidth_requests,
        }
    }
}
