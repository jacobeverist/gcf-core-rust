//! BitArrayBitvec - Prototype using bitvec crate.
//!
//! This is a validation prototype to compare performance with custom BitArray.
//! Focuses on API compatibility and critical operations for Phase 2.
//!
//! # Critical Requirements for Phase 2
//!
//! - Word-level access via `as_raw_slice()` / `as_raw_mut_slice()`
//! - Fast word copying (<120ns for 1024 bits)
//! - Fast equality comparison (<100ns for 1024 bits)
//! - Compatible API with custom BitArray
//!
//! # Design Notes
//!
//! - Uses `BitVec<u32, Lsb0>` for storage (matches custom implementation)
//! - Inline annotations on hot paths
//! - Word-level operations preferred for bulk copying

use bitvec::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::ops::{BitAnd, BitOr, BitXor, Not};

/// Word type for bit storage (32-bit unsigned integer)
pub type Word = u32;

/// Number of bits per word
pub const BITS_PER_WORD: usize = 32;

/// Prototype BitArray using bitvec crate.
///
/// This is a validation prototype to compare performance with custom implementation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BitArrayBitvec {
    /// Underlying bitvec storage with u32 words, LSB0 ordering
    bv: BitVec<u32, Lsb0>,
}

impl BitArrayBitvec {
    /// Create a new BitArrayBitvec with `n` bits, all initialized to 0.
    #[inline]
    pub fn new(n: usize) -> Self {
        Self {
            bv: BitVec::repeat(false, n),
        }
    }

    /// Resize the BitArrayBitvec to contain `n` bits.
    ///
    /// New bits are initialized to 0. If shrinking, excess bits are discarded.
    pub fn resize(&mut self, n: usize) {
        self.bv.resize(n, false);
    }

    /// Clear all storage and set size to 0.
    pub fn erase(&mut self) {
        self.bv.clear();
    }

    /// Get total number of bits.
    #[inline(always)]
    pub fn num_bits(&self) -> usize {
        self.bv.len()
    }

    /// Get number of words (CRITICAL for Phase 2).
    #[inline(always)]
    pub fn num_words(&self) -> usize {
        self.bv.as_raw_slice().len()
    }

    // =========================================================================
    // Single Bit Operations
    // =========================================================================

    /// Set bit at position `b` to 1.
    #[inline]
    pub fn set_bit(&mut self, b: usize) {
        debug_assert!(b < self.bv.len(), "bit index {} out of bounds (length: {})", b, self.bv.len());
        self.bv.set(b, true);
    }

    /// Get bit at position `b` (returns 0 or 1 as u8).
    #[inline]
    pub fn get_bit(&self, b: usize) -> u8 {
        debug_assert!(b < self.bv.len(), "bit index {} out of bounds (length: {})", b, self.bv.len());
        if self.bv[b] { 1 } else { 0 }
    }

    /// Clear bit at position `b` (set to 0).
    #[inline]
    pub fn clear_bit(&mut self, b: usize) {
        debug_assert!(b < self.bv.len(), "bit index {} out of bounds (length: {})", b, self.bv.len());
        self.bv.set(b, false);
    }

    /// Toggle bit at position `b` (0 -> 1, 1 -> 0).
    #[inline]
    pub fn toggle_bit(&mut self, b: usize) {
        debug_assert!(b < self.bv.len(), "bit index {} out of bounds (length: {})", b, self.bv.len());
        let current = self.bv[b];
        self.bv.set(b, !current);
    }

    /// Assign bit at position `b` to given value (0 or 1).
    #[inline]
    pub fn assign_bit(&mut self, b: usize, val: u8) {
        if val > 0 {
            self.set_bit(b);
        } else {
            self.clear_bit(b);
        }
    }

    // =========================================================================
    // Bulk Operations
    // =========================================================================

    /// Set all bits to 1.
    pub fn set_all(&mut self) {
        self.bv.fill(true);
    }

    /// Clear all bits to 0.
    pub fn clear_all(&mut self) {
        self.bv.fill(false);
    }

    /// Toggle all bits (binary NOT operation).
    pub fn toggle_all(&mut self) {
        for i in 0..self.bv.len() {
            let current = self.bv[i];
            self.bv.set(i, !current);
        }
    }

    /// Set range of bits [beg, beg+len) to 1.
    pub fn set_range(&mut self, beg: usize, len: usize) {
        debug_assert!(beg + len <= self.bv.len());
        for i in beg..(beg + len) {
            self.bv.set(i, true);
        }
    }

    /// Clear range of bits [beg, beg+len) to 0.
    pub fn clear_range(&mut self, beg: usize, len: usize) {
        debug_assert!(beg + len <= self.bv.len());
        for i in beg..(beg + len) {
            self.bv.set(i, false);
        }
    }

    // =========================================================================
    // Vector Operations (CRITICAL)
    // =========================================================================

    /// Set bits from vector of indices.
    ///
    /// Clears all bits first, then sets bits at indices in `idxs`.
    /// Indices >= num_bits are silently ignored.
    pub fn set_acts(&mut self, idxs: &[usize]) {
        self.clear_all();
        for &idx in idxs {
            if idx < self.bv.len() {
                self.bv.set(idx, true);
            }
        }
    }

    /// Get indices of all set bits.
    ///
    /// Returns a vector of indices where bits are 1, in ascending order.
    pub fn get_acts(&self) -> Vec<usize> {
        self.bv
            .iter_ones()
            .collect()
    }

    /// Get all bit values as vector of 0s and 1s.
    pub fn get_bits(&self) -> Vec<u8> {
        self.bv.iter().map(|b| if *b { 1 } else { 0 }).collect()
    }

    // =========================================================================
    // Counting Operations
    // =========================================================================

    /// Count number of set bits (population count).
    #[inline]
    pub fn num_set(&self) -> usize {
        self.bv.count_ones()
    }

    /// Count number of cleared bits.
    #[inline]
    pub fn num_cleared(&self) -> usize {
        self.bv.count_zeros()
    }

    /// Count number of similar set bits between two BitArrays.
    ///
    /// Returns count of bits that are 1 in both arrays (bitwise AND + popcount).
    pub fn num_similar(&self, other: &BitArrayBitvec) -> usize {
        assert_eq!(
            self.num_words(),
            other.num_words(),
            "BitArrayBitvec must have same word count"
        );

        let self_words = self.bv.as_raw_slice();
        let other_words = other.bv.as_raw_slice();

        self_words
            .iter()
            .zip(other_words.iter())
            .map(|(a, b)| (a & b).count_ones() as usize)
            .sum()
    }

    // =========================================================================
    // Search Operations
    // =========================================================================

    /// Find next set bit starting from position `beg`, with wrapping.
    ///
    /// Searches [beg, num_bits) then wraps to [0, beg).
    /// Returns Some(index) if found, None if no set bits exist.
    pub fn find_next_set_bit(&self, beg: usize) -> Option<usize> {
        debug_assert!(beg < self.bv.len());
        if self.bv.len() == 0 {
            return None;
        }

        // Search forward from beg
        for idx in self.bv[beg..].iter_ones() {
            return Some(beg + idx);
        }

        // Wrap around to beginning
        for idx in self.bv[..beg].iter_ones() {
            return Some(idx);
        }

        None
    }

    // =========================================================================
    // Random Operations
    // =========================================================================

    /// Randomly set exactly `n` bits to 1.
    ///
    /// Clears all bits first, then sets random bits.
    /// If n > num_bits, sets all bits.
    pub fn random_set_num<R: Rng>(&mut self, rng: &mut R, n: usize) {
        self.clear_all();
        let n_actual = n.min(self.bv.len());

        if n_actual == 0 {
            return;
        }

        // Simple algorithm: randomly pick indices until we have n unique ones
        let mut count = 0;
        while count < n_actual {
            let idx = rng.gen_range(0..self.bv.len());
            if !self.bv[idx] {
                self.bv.set(idx, true);
                count += 1;
            }
        }
    }

    /// Randomly set approximately `pct` percentage of bits to 1.
    ///
    /// Clears all bits first, then sets random bits.
    /// pct should be in range [0.0, 1.0].
    pub fn random_set_pct<R: Rng>(&mut self, rng: &mut R, pct: f64) {
        let n = ((self.bv.len() as f64) * pct).round() as usize;
        self.random_set_num(rng, n);
    }

    /// Fisher-Yates shuffle of active bit positions.
    pub fn random_shuffle<R: Rng>(&mut self, rng: &mut R) {
        // Get active indices
        let mut acts: Vec<usize> = self.get_acts();

        // Fisher-Yates shuffle
        for i in (1..acts.len()).rev() {
            let j = rng.gen_range(0..=i);
            acts.swap(i, j);
        }

        // Set shuffled positions
        self.set_acts(&acts);
    }

    // =========================================================================
    // Word-Level Access (CRITICAL for Phase 2)
    // =========================================================================

    /// Get direct read-only access to word storage.
    ///
    /// CRITICAL: Used for efficient word-level copying in Phase 2.
    #[inline(always)]
    pub fn words(&self) -> &[Word] {
        self.bv.as_raw_slice()
    }

    /// Get direct mutable access to word storage.
    ///
    /// CRITICAL: Used for efficient word-level copying in Phase 2.
    #[inline(always)]
    pub fn words_mut(&mut self) -> &mut [Word] {
        self.bv.as_raw_mut_slice()
    }

    // =========================================================================
    // Memory and Debug
    // =========================================================================

    /// Estimate memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() + self.bv.capacity() * std::mem::size_of::<Word>()
    }
}

// =============================================================================
// Operator Implementations
// =============================================================================

/// Bitwise AND operation
impl<'a, 'b> BitAnd<&'b BitArrayBitvec> for &'a BitArrayBitvec {
    type Output = BitArrayBitvec;

    fn bitand(self, rhs: &'b BitArrayBitvec) -> Self::Output {
        assert_eq!(
            self.bv.len(),
            rhs.bv.len(),
            "BitArrayBitvec AND: length mismatch"
        );

        let mut result = self.clone();
        result.bv &= &rhs.bv;
        result
    }
}

/// Bitwise OR operation
impl<'a, 'b> BitOr<&'b BitArrayBitvec> for &'a BitArrayBitvec {
    type Output = BitArrayBitvec;

    fn bitor(self, rhs: &'b BitArrayBitvec) -> Self::Output {
        assert_eq!(
            self.bv.len(),
            rhs.bv.len(),
            "BitArrayBitvec OR: length mismatch"
        );

        let mut result = self.clone();
        result.bv |= &rhs.bv;
        result
    }
}

/// Bitwise XOR operation
impl<'a, 'b> BitXor<&'b BitArrayBitvec> for &'a BitArrayBitvec {
    type Output = BitArrayBitvec;

    fn bitxor(self, rhs: &'b BitArrayBitvec) -> Self::Output {
        assert_eq!(
            self.bv.len(),
            rhs.bv.len(),
            "BitArrayBitvec XOR: length mismatch"
        );

        let mut result = self.clone();
        result.bv ^= &rhs.bv;
        result
    }
}

/// Bitwise NOT operation
impl<'a> Not for &'a BitArrayBitvec {
    type Output = BitArrayBitvec;

    fn not(self) -> Self::Output {
        let mut result = self.clone();
        result.toggle_all();
        result
    }
}

/// Equality comparison (CRITICAL for change tracking in Phase 2)
impl PartialEq for BitArrayBitvec {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.bv == other.bv
    }
}

impl Eq for BitArrayBitvec {}

// =============================================================================
// Helper Functions
// =============================================================================

/// Copy words between BitArrayBitvec instances.
///
/// CRITICAL: Must be as fast as custom implementation (<120ns for 1024 bits).
///
/// # Arguments
///
/// * `dst` - Destination BitArrayBitvec
/// * `src` - Source BitArrayBitvec
/// * `dst_word_offset` - Starting word index in destination
/// * `src_word_offset` - Starting word index in source
/// * `num_words` - Number of words to copy
#[inline(always)]
pub fn bitarray_copy_words_bitvec(
    dst: &mut BitArrayBitvec,
    src: &BitArrayBitvec,
    dst_word_offset: usize,
    src_word_offset: usize,
    num_words: usize,
) {
    let dst_start = dst_word_offset;
    let dst_end = dst_start + num_words;
    let src_start = src_word_offset;
    let src_end = src_start + num_words;

    debug_assert!(
        dst_end <= dst.num_words(),
        "dst word range out of bounds"
    );
    debug_assert!(
        src_end <= src.num_words(),
        "src word range out of bounds"
    );

    // Use word-level access for efficient copying
    let dst_words = dst.words_mut();
    let src_words = src.words();

    dst_words[dst_start..dst_end].copy_from_slice(&src_words[src_start..src_end]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_basic_creation() {
        let ba = BitArrayBitvec::new(1024);
        assert_eq!(ba.num_bits(), 1024);
        assert_eq!(ba.num_set(), 0);
        assert_eq!(ba.num_words(), 32); // 1024 / 32
    }

    #[test]
    fn test_set_get_bit() {
        let mut ba = BitArrayBitvec::new(1024);
        ba.set_bit(5);
        ba.set_bit(100);
        assert_eq!(ba.get_bit(5), 1);
        assert_eq!(ba.get_bit(100), 1);
        assert_eq!(ba.get_bit(10), 0);
        assert_eq!(ba.num_set(), 2);
    }

    #[test]
    fn test_set_acts_get_acts() {
        let mut ba = BitArrayBitvec::new(1024);
        ba.set_acts(&[5, 10, 15, 100, 500]);
        assert_eq!(ba.get_acts(), vec![5, 10, 15, 100, 500]);
        assert_eq!(ba.num_set(), 5);
    }

    #[test]
    fn test_word_level_access() {
        let ba = BitArrayBitvec::new(1024);
        assert_eq!(ba.num_words(), 32);
        let words = ba.words();
        assert_eq!(words.len(), 32);
    }

    #[test]
    fn test_word_level_copy() {
        let mut dst = BitArrayBitvec::new(1024);
        let mut src = BitArrayBitvec::new(1024);
        src.set_acts(&[5, 100, 500]);

        bitarray_copy_words_bitvec(&mut dst, &src, 0, 0, src.num_words());

        assert_eq!(dst.get_acts(), vec![5, 100, 500]);
        assert_eq!(dst.num_set(), 3);
    }

    #[test]
    fn test_partial_eq() {
        let mut ba1 = BitArrayBitvec::new(1024);
        let mut ba2 = BitArrayBitvec::new(1024);

        ba1.set_acts(&[5, 10]);
        ba2.set_acts(&[5, 10]);
        assert_eq!(ba1, ba2);

        ba2.set_bit(15);
        assert_ne!(ba1, ba2);
    }

    #[test]
    fn test_operators() {
        let mut ba1 = BitArrayBitvec::new(32);
        let mut ba2 = BitArrayBitvec::new(32);

        ba1.set_acts(&[0, 5, 10]);
        ba2.set_acts(&[5, 10, 15]);

        let result = &ba1 & &ba2;
        assert_eq!(result.get_acts(), vec![5, 10]);

        let result = &ba1 | &ba2;
        assert_eq!(result.get_acts(), vec![0, 5, 10, 15]);

        let result = &ba1 ^ &ba2;
        assert_eq!(result.get_acts(), vec![0, 15]);
    }

    #[test]
    fn test_num_similar() {
        let mut ba1 = BitArrayBitvec::new(1024);
        let mut ba2 = BitArrayBitvec::new(1024);

        ba1.set_acts(&[5, 10, 15, 20]);
        ba2.set_acts(&[10, 15, 20, 25]);

        assert_eq!(ba1.num_similar(&ba2), 3); // 10, 15, 20
    }

    #[test]
    fn test_random_operations() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let mut ba = BitArrayBitvec::new(1024);

        ba.random_set_num(&mut rng, 100);
        assert_eq!(ba.num_set(), 100);

        ba.random_set_pct(&mut rng, 0.1);
        assert!(ba.num_set() >= 95 && ba.num_set() <= 105);
    }
}
