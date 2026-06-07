//! Bloom filter for fast negative lookups.

/// A simple bloom filter using multiple hash functions.
#[derive(Debug, Clone)]
pub struct BloomFilter {
    bits: Vec<u64>,
    num_hashes: usize,
    num_bits: usize,
}

impl BloomFilter {
    /// Create a new bloom filter with the given capacity and false positive rate.
    /// `expected_items` is the expected number of items.
    /// `fp_rate` is the desired false positive rate (e.g., 0.01 for 1%).
    pub fn new(expected_items: usize, fp_rate: f64) -> Self {
        let num_bits = Self::optimal_num_bits(expected_items, fp_rate);
        let num_hashes = Self::optimal_num_hashes(num_bits, expected_items);
        let words = num_bits.div_ceil(64);
        BloomFilter {
            bits: vec![0u64; words],
            num_hashes,
            num_bits,
        }
    }

    /// Create with specific parameters.
    pub fn with_params(num_bits: usize, num_hashes: usize) -> Self {
        let words = num_bits.div_ceil(64);
        BloomFilter {
            bits: vec![0u64; words],
            num_hashes,
            num_bits,
        }
    }

    fn optimal_num_bits(n: usize, p: f64) -> usize {
        let m = -((n as f64) * p.ln()) / (2.0_f64.ln().powi(2));
        m.ceil() as usize
    }

    fn optimal_num_hashes(m: usize, n: usize) -> usize {
        let k = ((m as f64 / n.max(1) as f64) * 2.0_f64.ln()).ceil() as usize;
        k.clamp(1, 20)
    }

    fn hash_indices(&self, key: &[u8]) -> Vec<usize> {
        let mut indices = Vec::with_capacity(self.num_hashes);
        let h1 = Self::fnv1a(key);
        let h2 = Self::murmur_fallback(key);

        for i in 0..self.num_hashes {
            let hash = h1.wrapping_add((i as u64).wrapping_mul(h2));
            indices.push((hash as usize) % self.num_bits);
        }
        indices
    }

    fn fnv1a(data: &[u8]) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for &byte in data {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    fn murmur_fallback(data: &[u8]) -> u64 {
        let mut hash: u64 = 0x85ebca6b;
        for &byte in data {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0xcc9e2d51);
            hash = hash.rotate_left(13);
        }
        hash
    }

    /// Insert a key into the bloom filter.
    pub fn insert(&mut self, key: &[u8]) {
        for idx in self.hash_indices(key) {
            let word = idx / 64;
            let bit = idx % 64;
            self.bits[word] |= 1u64 << bit;
        }
    }

    /// Check if a key might be in the set.
    /// Returns false if the key is definitely not in the set.
    /// Returns true if the key might be in the set (could be false positive).
    pub fn might_contain(&self, key: &[u8]) -> bool {
        for idx in self.hash_indices(key) {
            let word = idx / 64;
            let bit = idx % 64;
            if self.bits[word] & (1u64 << bit) == 0 {
                return false;
            }
        }
        true
    }

    /// Return the number of bits in the filter.
    pub fn num_bits(&self) -> usize {
        self.num_bits
    }

    /// Return the number of hash functions.
    pub fn num_hashes(&self) -> usize {
        self.num_hashes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_check() {
        let mut bf = BloomFilter::new(100, 0.01);
        bf.insert(b"hello");
        assert!(bf.might_contain(b"hello"));
    }

    #[test]
    fn test_negative() {
        let mut bf = BloomFilter::new(1000, 0.01);
        bf.insert(b"exists");
        // Most non-existent keys should return false
        let mut false_positives = 0;
        for i in 0..1000 {
            if bf.might_contain(format!("nonexist_{}", i).as_bytes()) {
                false_positives += 1;
            }
        }
        // Should be well under 5%
        assert!(false_positives < 50, "Too many false positives: {}", false_positives);
    }

    #[test]
    fn test_empty_filter() {
        let bf = BloomFilter::new(100, 0.01);
        assert!(!bf.might_contain(b"anything"));
    }

    #[test]
    fn test_fp_rate() {
        let mut bf = BloomFilter::new(1000, 0.01);
        for i in 0..1000 {
            bf.insert(format!("key_{}", i).as_bytes());
        }
        let mut false_pos = 0;
        let test_count = 10000;
        for i in 0..test_count {
            if bf.might_contain(format!("nokey_{}", i).as_bytes()) {
                false_pos += 1;
            }
        }
        let rate = false_pos as f64 / test_count as f64;
        assert!(rate < 0.05, "FP rate too high: {}", rate);
    }

    #[test]
    fn test_all_present() {
        let mut bf = BloomFilter::new(100, 0.01);
        for i in 0..100 {
            bf.insert(format!("k{}", i).as_bytes());
        }
        for i in 0..100 {
            assert!(bf.might_contain(format!("k{}", i).as_bytes()));
        }
    }

    #[test]
    fn test_small_filter() {
        let mut bf = BloomFilter::with_params(64, 3);
        bf.insert(b"a");
        bf.insert(b"b");
        assert!(bf.might_contain(b"a"));
        assert!(bf.might_contain(b"b"));
        assert!(!bf.might_contain(b"c"));
    }
}
