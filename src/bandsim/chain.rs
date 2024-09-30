use std::collections::BTreeMap;
use std::fmt::Debug;

use crate::bandsim::bandwidth_request::BandwidthRequest;

/// Maximum number of bytes that a shard can send or receive at a single height
pub const MAX_SHARD_BANDWIDTH: usize = 4_500_000;

/// Minimum size of a single receipt
pub const MIN_RECEIPT_SIZE: usize = 1_000;

/// Maximum size of a single receipt
pub const MAX_RECEIPT_SIZE: usize = 4_000_000;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShardUId {
    pub version: u32,
    pub shard_id: u32,
}

impl ShardUId {
    pub fn new(id: usize) -> ShardUId {
        ShardUId {
            version: 0,
            shard_id: id as u32,
        }
    }
}
impl Debug for ShardUId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.version != 0 {
            panic!("You actually used the version field!");
        }
        write!(f, "shard_{}", self.shard_id)
    }
}

/// A link between two shards.
/// Receipts are sent `from` some shard `to` some shard over some ShardLink.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShardLink {
    pub from: ShardUId,
    pub to: ShardUId,
}

impl ShardLink {
    pub fn is(&self, a: usize, b: usize) -> bool {
        self.from == ShardUId::new(a) && self.to == ShardUId::new(b)
    }
}

impl Debug for ShardLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?} -> {:?}]", self.from, self.to)
    }
}

pub struct Chunk {
    pub prev_incoming_receipts_size: usize,
    pub prev_outgoing_receipts_size: BTreeMap<ShardUId, usize>,
    pub bandwidth_requests: Vec<BandwidthRequest>,
}

pub struct Block {
    pub height: usize,
    pub chunks: BTreeMap<ShardUId, Option<Chunk>>,
}

pub struct Receipt {
    pub size: usize,
}
