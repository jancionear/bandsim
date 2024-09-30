use std::collections::VecDeque;

use crate::bandsim::bandwidth_request::BandwidthRequest;
use crate::bandsim::chain::{Receipt, ShardUId, MAX_SHARD_BANDWIDTH};

pub struct OutgoingQueue {
    to_shard: ShardUId,
    receipts: VecDeque<Receipt>,
    total_size: usize,
}

impl OutgoingQueue {
    pub fn new(to_shard: ShardUId) -> Self {
        OutgoingQueue {
            to_shard,
            receipts: VecDeque::new(),
            total_size: 0,
        }
    }

    pub fn push(&mut self, receipt: Receipt) {
        self.total_size += receipt.size;
        self.receipts.push_back(receipt);
    }

    pub fn pop(&mut self) -> Option<Receipt> {
        let res = self.receipts.pop_front();
        res.as_ref()
            .inspect(|receipt| self.total_size -= receipt.size);
        res
    }

    pub fn first_receipt_size(&self) -> Option<usize> {
        self.receipts.front().map(|r| r.size)
    }

    pub fn total_size(&self) -> usize {
        self.total_size
    }

    pub fn make_bandwidth_request(&self, base_bandwidth: usize) -> Option<BandwidthRequest> {
        BandwidthRequest::from_receipt_sizes(
            self.to_shard,
            self.receipts.iter().map(|r| r.size),
            base_bandwidth,
            MAX_SHARD_BANDWIDTH,
        )
    }

    pub fn is_empty(&self) -> bool {
        self.receipts.is_empty()
    }
}
