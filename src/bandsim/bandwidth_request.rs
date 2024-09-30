use crate::bandsim::chain::{ShardUId, MAX_RECEIPT_SIZE, MAX_SHARD_BANDWIDTH};

const BANDWIDTH_REQUEST_VALUES_NUM: usize = 40;

#[derive(Clone, Debug)]
pub struct BandwidthRequest {
    pub to_shard: ShardUId,
    pub grant_options_bitmap: BandwidthRequestBitmap,
}

impl BandwidthRequest {
    pub fn from_receipt_sizes(
        to_shard: ShardUId,
        receipt_sizes: impl Iterator<Item = usize>,
        base_bandwidth: usize,
        max_bandwidth: usize,
    ) -> Option<BandwidthRequest> {
        let values = BandwidthRequestValues::new(base_bandwidth, max_bandwidth);
        let mut bitmap = BandwidthRequestBitmap::new();

        let mut total_size = 0;
        let mut cur_value = 0;
        for receipt_size in receipt_sizes {
            total_size += receipt_size;

            if total_size <= base_bandwidth {
                continue;
            }

            // Find a value that is at least as big as the total_size
            while cur_value < values.0.len() && values.0[cur_value] < total_size {
                cur_value += 1;
            }

            if cur_value == values.0.len() {
                bitmap.set_bit(bitmap.len() - 1, true);
                break;
            }

            // Request the value thath is at least as large as total_size
            bitmap.set_bit(cur_value, true);
        }

        if bitmap.is_all_false() {
            return None;
        }

        Some(BandwidthRequest {
            to_shard,
            grant_options_bitmap: bitmap,
        })
    }
}

/// Bandwidth values that can be requested in a BandwidthRequest.
/// nth bit in the bitmap is set when the shard requests the nth value as one of the options.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BandwidthRequestValues(pub [usize; BANDWIDTH_REQUEST_VALUES_NUM]);

impl BandwidthRequestValues {
    pub fn new(base_bandwidth: usize, max_bandwidth: usize) -> BandwidthRequestValues {
        assert_eq!(max_bandwidth, MAX_SHARD_BANDWIDTH);
        // values[-1] = base_bandwidth
        // values[values.len() - 1] = max_bandwidth
        // values[i] = linear interpolation between values[-1] and values[values.len() - 1]
        let mut values = [0; BANDWIDTH_REQUEST_VALUES_NUM];
        for i in 0..values.len() {
            values[i] = base_bandwidth + (max_bandwidth - base_bandwidth) * (i + 1) / values.len();
        }

        // The value that is closest to MAX_RECEIPT_SIZE is set to MAX_RECEIPT_SIZE.
        // This ensures that the value corresponding to max size receipts can be granted after base bandwidth is granted.
        let mut closest_to_max: usize = 0;
        for value in &values {
            if value.abs_diff(MAX_RECEIPT_SIZE) < closest_to_max.abs_diff(MAX_RECEIPT_SIZE) {
                closest_to_max = *value;
            }
        }
        for value in values.iter_mut() {
            if *value == closest_to_max {
                *value = MAX_RECEIPT_SIZE;
            }
        }

        let values_sorted = values.windows(2).all(|w| w[0] < w[1]);
        if !values_sorted {
            panic!(
                "Values not sorted! base_bandwidth = {}, max_bandwidth = {}",
                base_bandwidth, max_bandwidth
            );
        }

        BandwidthRequestValues(values)
    }
}

const BANDWIDTH_REQUEST_BITMAP_ARRAY_SIZE: usize =
    BANDWIDTH_REQUEST_VALUES_NUM / 8 + BANDWIDTH_REQUEST_VALUES_NUM % 8;

#[allow(clippy::len_without_is_empty)]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct BandwidthRequestBitmap([u8; BANDWIDTH_REQUEST_BITMAP_ARRAY_SIZE]);

impl BandwidthRequestBitmap {
    pub fn new() -> BandwidthRequestBitmap {
        BandwidthRequestBitmap([0; BANDWIDTH_REQUEST_BITMAP_ARRAY_SIZE])
    }

    pub fn set_bit(&mut self, index: usize, value: bool) {
        if index >= BANDWIDTH_REQUEST_VALUES_NUM {
            panic!(
                "set_bit index out of bounds: {} >= {}",
                index, BANDWIDTH_REQUEST_VALUES_NUM
            );
        }

        let byte = &mut self.0[index / 8];
        let bit_num = index % 8;
        if value {
            *byte |= 1_u8 << bit_num;
        } else {
            *byte &= !(1u8 << bit_num);
        }
    }

    pub fn get_bit(&self, index: usize) -> bool {
        if index >= BANDWIDTH_REQUEST_VALUES_NUM {
            panic!(
                "get_bit index out of bounds: {} >= {}",
                index, BANDWIDTH_REQUEST_VALUES_NUM
            );
        }

        let byte = self.0[index / 8];
        let bit_num = index % 8;
        ((byte >> bit_num) & 1u8) == 1u8
    }

    pub fn len(&self) -> usize {
        BANDWIDTH_REQUEST_VALUES_NUM
    }

    pub fn is_all_false(&self) -> bool {
        self.0.iter().all(|b| *b == 0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BandwidthRequestOptions(pub Vec<usize>);

impl BandwidthRequestOptions {
    pub fn from_bitmap(
        bitmap: &BandwidthRequestBitmap,
        base_bandwidth: usize,
        max_bandwidth: usize,
    ) -> BandwidthRequestOptions {
        let values = BandwidthRequestValues::new(base_bandwidth, max_bandwidth);
        let mut options = Vec::new();
        for i in 0..bitmap.len() {
            if bitmap.get_bit(i) {
                options.push(values.0[i]);
            }
        }
        BandwidthRequestOptions(options)
    }
}

pub mod tests {
    use rand::seq::SliceRandom;
    use rand::Rng;

    use crate::bandsim::rng::rng_from_seed;

    use super::{BandwidthRequestBitmap, BANDWIDTH_REQUEST_VALUES_NUM};

    #[test]
    fn test_bandwidth_request_bitmap() {
        let mut bitmap = BandwidthRequestBitmap::new();
        assert_eq!(bitmap.len(), BANDWIDTH_REQUEST_VALUES_NUM);

        let mut fake_bitmap = vec![false; bitmap.len()];

        let mut rng = rng_from_seed(0);

        for _ in 0..1000 {
            let index = rng.gen_range(0..bitmap.len());
            let value = rng.gen_bool(0.5);

            bitmap.set_bit(index, value);
            fake_bitmap[index] = value;

            let mut indexes: Vec<usize> = (0..bitmap.len()).collect();
            indexes.shuffle(&mut rng);
            for i in indexes {
                assert_eq!(bitmap.get_bit(i), fake_bitmap[i]);
            }
        }
    }
}
