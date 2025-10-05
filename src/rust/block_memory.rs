//! BlockMemory - Synaptic learning mechanisms with dendrites and receptors.
//!
//! This module provides the `BlockMemory` structure that implements synaptic-like
//! learning with dendrites, receptors, and permanence-based plasticity. Inspired
//! by biological dendrites that detect patterns in their receptive fields.
//!
//! # Architecture
//!
//! - **Dendrites** - Computational units that detect patterns (num_d)
//! - **Receptors per dendrite** - Connection points to inputs (num_rpd)
//! - **Receptor addresses** - Which input bits each receptor connects to (r_addrs)
//! - **Receptor permanences** - Connection strengths 0-99 (r_perms)
//! - **Dendrite connections** - Optional connectivity mask (d_conns)
//!
//! # Learning Parameters
//!
//! - `perm_thr` - Permanence threshold for "connected" (typically 20/99)
//! - `perm_inc` - Permanence increment on positive learning (typically 2)
//! - `perm_dec` - Permanence decrement on negative learning (typically 1)
//! - `pct_learn` - Percentage of receptors that can learn per update (typically 0.3)
//!
//! # Core Operations
//!
//! - `overlap(d, input)` - Count matching connected receptors
//! - `learn(d, input)` - Strengthen matching, weaken non-matching
//! - `punish(d, input)` - Weaken matching receptors
//! - `learn_move(d, input)` - Move dead receptors to new positions
//!
//! # Examples
//!
//! ```
//! use gnomics::BlockMemory;
//! use gnomics::BitArray;
//! use rand::SeedableRng;
//! use rand::rngs::StdRng;
//!
//! let mut memory = BlockMemory::new(100, 50, 20, 2, 1, 0.3);
//! let mut rng = StdRng::seed_from_u64(42);
//!
//! // Initialize with pooled connectivity
//! memory.init_pooled(1024, &mut rng, 0.8, 0.5);
//!
//! // Create input pattern
//! let mut input = BitArray::new(1024);
//! input.set_bit(10);
//! input.set_bit(20);
//! input.set_bit(30);
//!
//! // Compute overlap for dendrite 0
//! let overlap = memory.overlap(0, &input);
//!
//! // Learn pattern on dendrite 0
//! memory.learn(0, &input, &mut rng);
//! ```

use crate::bitarray::BitArray;
use crate::utils::{max, min};
use rand::rngs::StdRng;
use rand::Rng;

/// Minimum permanence value
pub const PERM_MIN: u8 = 0;

/// Maximum permanence value
pub const PERM_MAX: u8 = 99;

/// BlockMemory implements synaptic learning with dendrites and receptors.
///
/// Each dendrite has `num_rpd` receptors that connect to different positions
/// in the input space. Receptor permanences slowly adapt via Hebbian-like learning.
pub struct BlockMemory {
    /// Dendrite activation state (1=active, 0=inactive)
    pub state: BitArray,

    // Parameters
    num_i: usize,     // Number of input bits
    num_d: usize,     // Number of dendrites
    num_rpd: usize,   // Receptors per dendrite
    num_r: usize,     // Total receptors (num_d * num_rpd)
    perm_thr: u8,     // Permanence threshold (0-99)
    perm_inc: u8,     // Permanence increment (0-99)
    perm_dec: u8,     // Permanence decrement (0-99)
    pct_learn: f64,   // Learning percentage (0.0-1.0)

    // Arrays
    r_addrs: Vec<usize>, // Receptor addresses (flattened 2D: [num_d][num_rpd])
    r_perms: Vec<u8>,    // Receptor permanences (flattened 2D: [num_d][num_rpd])
    d_conns: Vec<BitArray>, // Optional dendrite connections (for fast overlap)
    lmask: BitArray,     // Learning mask (which receptors can learn)

    // Flags
    init_flag: bool,
    conns_flag: bool, // Using connection BitArrays?
}

impl BlockMemory {
    /// Create a new BlockMemory with specified parameters.
    ///
    /// Must call `init()` or `init_pooled()` before use.
    ///
    /// # Arguments
    ///
    /// * `num_d` - Number of dendrites
    /// * `num_rpd` - Receptors per dendrite (can be 0 if using init_pooled)
    /// * `perm_thr` - Permanence threshold (0-99, typically 20)
    /// * `perm_inc` - Permanence increment (0-99, typically 2)
    /// * `perm_dec` - Permanence decrement (0-99, typically 1)
    /// * `pct_learn` - Learning percentage (0.0-1.0, typically 0.3)
    pub fn new(
        num_d: usize,
        num_rpd: usize,
        perm_thr: u8,
        perm_inc: u8,
        perm_dec: u8,
        pct_learn: f64,
    ) -> Self {
        assert!(num_d > 0, "num_d must be > 0");
        assert!(perm_thr <= PERM_MAX);
        assert!(perm_inc <= PERM_MAX);
        assert!(perm_dec <= PERM_MAX);
        assert!((0.0..=1.0).contains(&pct_learn), "pct_learn must be 0.0-1.0");

        let num_r = num_d * num_rpd;

        Self {
            state: BitArray::new(num_d),
            num_i: 0,
            num_d,
            num_rpd,
            num_r,
            perm_thr,
            perm_inc,
            perm_dec,
            pct_learn,
            r_addrs: vec![0; num_r],
            r_perms: vec![0; num_r],
            d_conns: Vec::new(),
            lmask: BitArray::new(num_rpd),
            init_flag: false,
            conns_flag: false,
        }
    }

    /// Initialize with full connectivity (all receptors address random inputs).
    ///
    /// # Arguments
    ///
    /// * `num_i` - Number of input bits
    /// * `rng` - Random number generator
    pub fn init(&mut self, num_i: usize, rng: &mut StdRng) {
        assert!(num_i > 0, "num_i must be > 0");

        self.num_i = num_i;

        // Setup learning mask (first pct_learn receptors can learn)
        let num_learn = (self.num_rpd as f64 * self.pct_learn) as usize;
        self.lmask.clear_all();
        self.lmask.set_range(0, num_learn);

        // Initialize random addresses and zero permanences
        for addr in self.r_addrs.iter_mut() {
            *addr = rng.gen_range(0..num_i);
        }
        self.r_perms.fill(0);

        self.init_flag = true;
    }

    /// Initialize with optional connection BitArrays.
    ///
    /// Connection BitArrays enable fast `overlap_conn()` via `num_similar()`.
    pub fn init_conn(&mut self, num_i: usize, rng: &mut StdRng) {
        self.init(num_i, rng);

        // Allocate connection BitArrays
        self.d_conns.clear();
        self.d_conns.resize(self.num_d, BitArray::new(num_i));

        self.conns_flag = true;
    }

    /// Initialize with pooled (sparse) connectivity.
    ///
    /// Each dendrite samples a random subset of input space.
    ///
    /// # Arguments
    ///
    /// * `num_i` - Number of input bits
    /// * `rng` - Random number generator
    /// * `pct_pool` - Pooling percentage (0.0-1.0, typically 0.8 for 80% sparsity)
    /// * `pct_conn` - Initially connected percentage (0.0-1.0, typically 0.5)
    ///
    /// # Examples
    ///
    /// ```
    /// use gnomics::BlockMemory;
    /// use rand::SeedableRng;
    /// use rand::rngs::StdRng;
    ///
    /// let mut memory = BlockMemory::new(100, 0, 20, 2, 1, 0.3);
    /// let mut rng = StdRng::seed_from_u64(42);
    ///
    /// // Each dendrite samples 80% of 1024 input bits
    /// // 50% initially connected
    /// memory.init_pooled(1024, &mut rng, 0.8, 0.5);
    /// ```
    pub fn init_pooled(
        &mut self,
        num_i: usize,
        rng: &mut StdRng,
        pct_pool: f64,
        pct_conn: f64,
    ) {
        assert!(num_i > 0, "num_i must be > 0");
        assert!((0.0..=1.0).contains(&pct_pool), "pct_pool must be 0.0-1.0");
        assert!((0.0..=1.0).contains(&pct_conn), "pct_conn must be 0.0-1.0");

        self.num_i = num_i;

        // Recalculate num_rpd based on pooling
        self.num_rpd = (num_i as f64 * pct_pool) as usize;
        self.num_r = self.num_d * self.num_rpd;

        // Resize arrays
        self.r_addrs.clear();
        self.r_addrs.resize(self.num_r, 0);
        self.r_perms.clear();
        self.r_perms.resize(self.num_r, 0);
        self.lmask.resize(self.num_rpd);

        // Setup learning mask
        let num_learn = (self.num_rpd as f64 * self.pct_learn) as usize;
        self.lmask.clear_all();
        self.lmask.set_range(0, num_learn);

        // Initialize each dendrite
        let num_init = (self.num_rpd as f64 * pct_conn) as usize;
        let mut rand_addrs: Vec<usize> = (0..num_i).collect();

        for d in 0..self.num_d {
            // Shuffle addresses for this dendrite
            crate::utils::shuffle_indices(&mut rand_addrs, rng);

            let r_beg = d * self.num_rpd;
            let r_end = r_beg + self.num_rpd;

            for (j, r) in (r_beg..r_end).enumerate() {
                self.r_addrs[r] = rand_addrs[j];

                // First pct_conn receptors start connected
                if j < num_init {
                    self.r_perms[r] = self.perm_thr; // Connected
                } else {
                    self.r_perms[r] = self.perm_thr.saturating_sub(1); // Just below threshold
                }
            }
        }

        self.init_flag = true;
    }

    /// Initialize pooled with connection BitArrays.
    pub fn init_pooled_conn(
        &mut self,
        num_i: usize,
        rng: &mut StdRng,
        pct_pool: f64,
        pct_conn: f64,
    ) {
        self.init_pooled(num_i, rng, pct_pool, pct_conn);

        // Allocate and update connection BitArrays
        self.d_conns.clear();
        self.d_conns.resize(self.num_d, BitArray::new(num_i));

        self.conns_flag = true;  // Set flag BEFORE calling update_conns

        for d in 0..self.num_d {
            self.update_conns(d);
        }
    }

    /// Compute overlap between dendrite and input.
    ///
    /// Returns count of receptors that are both:
    /// 1. Connected (permanence >= threshold)
    /// 2. Active (input bit is 1 at receptor address)
    ///
    /// # Arguments
    ///
    /// * `d` - Dendrite index
    /// * `input` - Input BitArray
    ///
    /// # Returns
    ///
    /// Overlap score (0 to num_rpd)
    #[inline]
    pub fn overlap(&self, d: usize, input: &BitArray) -> usize {
        assert!(self.init_flag, "must call init() first");
        assert!(d < self.num_d, "dendrite index out of bounds");

        let mut overlap = 0;
        let r_beg = d * self.num_rpd;
        let r_end = r_beg + self.num_rpd;

        for r in r_beg..r_end {
            // Connected and active?
            if self.r_perms[r] >= self.perm_thr && input.get_bit(self.r_addrs[r]) > 0 {
                overlap += 1;
            }
        }

        overlap
    }

    /// Compute overlap using connection BitArray (faster for large inputs).
    ///
    /// Requires `init_conn()` or `init_pooled_conn()`.
    #[inline]
    pub fn overlap_conn(&self, d: usize, input: &BitArray) -> usize {
        assert!(self.init_flag && self.conns_flag);
        assert!(d < self.num_d);

        self.d_conns[d].num_similar(input)
    }

    /// Learn pattern on dendrite.
    ///
    /// Updates receptor permanences:
    /// - Increment if input bit is active at receptor address
    /// - Decrement if input bit is inactive
    /// - Only updates receptors selected by learning mask
    ///
    /// # Arguments
    ///
    /// * `d` - Dendrite index
    /// * `input` - Input pattern
    /// * `rng` - RNG for shuffling learning mask
    pub fn learn(&mut self, d: usize, input: &BitArray, rng: &mut StdRng) {
        assert!(self.init_flag);
        assert!(d < self.num_d);

        // Shuffle learning mask if not learning 100%
        if self.pct_learn < 1.0 {
            self.lmask.random_shuffle(rng);
        }

        let r_beg = d * self.num_rpd;
        let r_end = r_beg + self.num_rpd;

        for (l, r) in (r_beg..r_end).enumerate() {
            if self.lmask.get_bit(l) > 0 {
                let addr = self.r_addrs[r];

                if input.get_bit(addr) > 0 {
                    // Active: increment permanence
                    self.r_perms[r] = min(self.r_perms[r] + self.perm_inc, PERM_MAX);
                } else {
                    // Inactive: decrement permanence
                    self.r_perms[r] = max(self.r_perms[r], self.perm_dec) - self.perm_dec;
                }
            }
        }
    }

    /// Learn with connection BitArray update.
    pub fn learn_conn(&mut self, d: usize, input: &BitArray, rng: &mut StdRng) {
        assert!(self.conns_flag);
        self.learn(d, input, rng);
        self.update_conns(d);
    }

    /// Learn and move dead receptors.
    ///
    /// Similar to `learn()` but when a receptor permanence hits zero, it is
    /// moved to a new random active input bit and reset to threshold.
    ///
    /// This maximizes receptor usage and prevents permanent dead receptors.
    pub fn learn_move(&mut self, d: usize, input: &BitArray, rng: &mut StdRng) {
        assert!(self.init_flag);
        assert!(d < self.num_d);

        // Random starting address for receptor movement
        let mut next_addr = rng.gen_range(0..self.num_i);

        // Shuffle learning mask
        if self.pct_learn < 1.0 {
            self.lmask.random_shuffle(rng);
        }

        let r_beg = d * self.num_rpd;
        let r_end = r_beg + self.num_rpd;

        // Find available input bits (not already covered by receptors)
        let mut available = input.clone();
        for r in r_beg..r_end {
            if self.r_perms[r] > 0 {
                available.clear_bit(self.r_addrs[r]);
            }
        }

        // Learn
        for (l, r) in (r_beg..r_end).enumerate() {
            if self.lmask.get_bit(l) > 0 {
                if self.r_perms[r] > 0 {
                    // Normal learning
                    let addr = self.r_addrs[r];
                    if input.get_bit(addr) > 0 {
                        self.r_perms[r] = min(self.r_perms[r] + self.perm_inc, PERM_MAX);
                    } else {
                        self.r_perms[r] = max(self.r_perms[r], self.perm_dec) - self.perm_dec;
                    }
                } else {
                    // Move receptor to new active bit
                    let mut search_addr = next_addr;
                    loop {
                        if available.get_bit(search_addr) > 0 {
                            self.r_addrs[r] = search_addr;
                            self.r_perms[r] = self.perm_thr;
                            available.clear_bit(search_addr);
                            next_addr = rng.gen_range(0..self.num_i);
                            break;
                        }
                        search_addr = (search_addr + 1) % self.num_i;
                        if search_addr == next_addr {
                            break; // Searched all, none available
                        }
                    }
                }
            }
        }
    }

    /// Learn and move with connection update.
    pub fn learn_move_conn(&mut self, d: usize, input: &BitArray, rng: &mut StdRng) {
        assert!(self.conns_flag);
        self.learn_move(d, input, rng);
        self.update_conns(d);
    }

    /// Punish dendrite (decrease permanences for active receptors).
    ///
    /// Decrements permanence for receptors connected to active input bits.
    /// Used for negative learning (e.g., penalize false positives).
    pub fn punish(&mut self, d: usize, input: &BitArray, rng: &mut StdRng) {
        assert!(self.init_flag);
        assert!(d < self.num_d);

        // Shuffle learning mask
        if self.pct_learn < 1.0 {
            self.lmask.random_shuffle(rng);
        }

        let r_beg = d * self.num_rpd;
        let r_end = r_beg + self.num_rpd;

        for (l, r) in (r_beg..r_end).enumerate() {
            if self.lmask.get_bit(l) > 0 {
                let addr = self.r_addrs[r];
                if input.get_bit(addr) > 0 {
                    // Active: decrement by perm_inc (stronger punishment)
                    self.r_perms[r] = max(self.r_perms[r], self.perm_inc) - self.perm_inc;
                }
            }
        }
    }

    /// Punish with connection update.
    pub fn punish_conn(&mut self, d: usize, input: &BitArray, rng: &mut StdRng) {
        assert!(self.conns_flag);
        self.punish(d, input, rng);
        self.update_conns(d);
    }

    /// Clear dendrite activation state.
    pub fn clear(&mut self) {
        self.state.clear_all();
    }

    /// Get number of dendrites.
    #[inline]
    pub fn num_dendrites(&self) -> usize {
        self.num_d
    }

    /// Get receptor addresses for a dendrite.
    pub fn addrs(&self, d: usize) -> Vec<usize> {
        assert!(self.init_flag);
        assert!(d < self.num_d);

        let r_beg = d * self.num_rpd;
        let r_end = r_beg + self.num_rpd;

        self.r_addrs[r_beg..r_end].to_vec()
    }

    /// Get receptor permanences for a dendrite.
    pub fn perms(&self, d: usize) -> Vec<u8> {
        assert!(self.init_flag);
        assert!(d < self.num_d);

        let r_beg = d * self.num_rpd;
        let r_end = r_beg + self.num_rpd;

        self.r_perms[r_beg..r_end].to_vec()
    }

    /// Get connection BitArray for a dendrite (if using connections).
    pub fn conns(&self, d: usize) -> Option<&BitArray> {
        if self.conns_flag {
            Some(&self.d_conns[d])
        } else {
            None
        }
    }

    /// Estimate memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        let mut bytes = std::mem::size_of::<Self>();

        bytes += self.state.memory_usage();
        bytes += self.r_addrs.capacity() * std::mem::size_of::<usize>();
        bytes += self.r_perms.capacity() * std::mem::size_of::<u8>();
        bytes += self.lmask.memory_usage();

        if self.conns_flag && !self.d_conns.is_empty() {
            bytes += self.d_conns.len() * self.d_conns[0].memory_usage();
        }

        bytes
    }

    /// Update connection BitArray for a dendrite.
    ///
    /// Sets bits for all connected receptors (permanence >= threshold).
    fn update_conns(&mut self, d: usize) {
        assert!(self.conns_flag);
        assert!(d < self.num_d);

        let r_beg = d * self.num_rpd;
        let r_end = r_beg + self.num_rpd;

        self.d_conns[d].clear_all();

        for r in r_beg..r_end {
            if self.r_perms[r] >= self.perm_thr {
                self.d_conns[d].set_bit(self.r_addrs[r]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_new() {
        let memory = BlockMemory::new(100, 50, 20, 2, 1, 0.3);
        assert_eq!(memory.num_dendrites(), 100);
        assert!(!memory.init_flag);
    }

    #[test]
    fn test_init() {
        let mut memory = BlockMemory::new(10, 20, 20, 2, 1, 0.3);
        let mut rng = StdRng::seed_from_u64(42);

        memory.init(100, &mut rng);

        assert!(memory.init_flag);
        assert_eq!(memory.r_addrs.len(), 10 * 20);
    }

    #[test]
    fn test_init_pooled() {
        let mut memory = BlockMemory::new(10, 0, 20, 2, 1, 0.3);
        let mut rng = StdRng::seed_from_u64(42);

        memory.init_pooled(1000, &mut rng, 0.5, 0.5);

        assert!(memory.init_flag);
        // 50% of 1000 = 500 receptors per dendrite
        assert_eq!(memory.num_rpd, 500);
    }

    #[test]
    fn test_overlap() {
        let mut memory = BlockMemory::new(1, 10, 20, 2, 1, 1.0);
        let mut rng = StdRng::seed_from_u64(42);

        memory.init_pooled(100, &mut rng, 0.5, 1.0); // All connected

        let mut input = BitArray::new(100);

        // Get addresses for dendrite 0
        let addrs = memory.addrs(0);

        // Activate first 5 addresses
        for &addr in addrs.iter().take(5) {
            input.set_bit(addr);
        }

        let overlap = memory.overlap(0, &input);
        assert_eq!(overlap, 5);
    }

    #[test]
    fn test_learn() {
        let mut memory = BlockMemory::new(1, 10, 20, 2, 1, 1.0);
        let mut rng = StdRng::seed_from_u64(42);

        memory.init(100, &mut rng);

        // Set initial permanences
        for i in 0..10 {
            memory.r_perms[i] = 20;
            memory.r_addrs[i] = i * 10;
        }

        let mut input = BitArray::new(100);
        input.set_bit(0);  // Matches r_addrs[0]
        input.set_bit(10); // Matches r_addrs[1]

        let perms_before = memory.perms(0);

        memory.learn(0, &input, &mut rng);

        let perms_after = memory.perms(0);

        // First two should increase
        assert!(perms_after[0] > perms_before[0]);
        assert!(perms_after[1] > perms_before[1]);

        // Rest should decrease
        assert!(perms_after[2] < perms_before[2]);
    }

    #[test]
    fn test_punish() {
        let mut memory = BlockMemory::new(1, 10, 20, 2, 1, 1.0);
        let mut rng = StdRng::seed_from_u64(42);

        memory.init(100, &mut rng);

        // Set initial permanences
        for i in 0..10 {
            memory.r_perms[i] = 20;
            memory.r_addrs[i] = i * 10;
        }

        let mut input = BitArray::new(100);
        input.set_bit(0);

        let perms_before = memory.perms(0);

        memory.punish(0, &input, &mut rng);

        let perms_after = memory.perms(0);

        // First should decrease
        assert!(perms_after[0] < perms_before[0]);

        // Rest should stay same (not active)
        assert_eq!(perms_after[1], perms_before[1]);
    }

    #[test]
    fn test_memory_usage() {
        let mut memory = BlockMemory::new(100, 50, 20, 2, 1, 0.3);
        let mut rng = StdRng::seed_from_u64(42);

        memory.init(1000, &mut rng);

        let usage = memory.memory_usage();
        assert!(usage > 0);
    }
}
