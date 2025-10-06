//! BitArray - Efficient bit manipulation using 32-bit words.
//!
//! This module provides a high-performance bit array implementation that stores
//! bits in packed 32-bit words, providing 32× compression compared to byte arrays.
//!
//! # Design
//!
//! - Uses `Vec<u32>` for storage (32-bit words)
//! - Bit indexing: word_idx = bit_idx / 32, bit_offset = bit_idx % 32
//! - Optimized for bulk operations and word-level copying
//! - Critical for Phase 2 lazy copying in `BlockInput::pull()`
//!
//! # Examples
//!
//! ```
//! use gnomics::BitArray;
//!
//! let mut ba = BitArray::new(1024);
//! ba.set_bit(5);
//! ba.set_bit(10);
//! assert_eq!(ba.num_set(), 2);
//! assert_eq!(ba.get_acts(), vec![5, 10]);
//! ```

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::ops::{BitAnd, BitOr, BitXor, Not};

/// Word type for bit storage (32-bit unsigned integer)
pub type Word = u32;

/// Number of bits per word
pub const BITS_PER_WORD: usize = 32;

/// Maximum word value
pub const WORD_MAX: Word = Word::MAX;

/// Get word index from bit position
#[inline(always)]
const fn get_word_idx(bit_pos: usize) -> usize {
    bit_pos >> 5 // bit_pos / 32
}

/// Get bit index within word from bit position
#[inline(always)]
const fn get_bit_idx(bit_pos: usize) -> usize {
    bit_pos & 31 // bit_pos % 32
}

/// Create bitmask with n bits set (from LSB)
#[inline(always)]
const fn bitmask(n: usize) -> Word {
    if n == 0 {
        0
    } else if n >= BITS_PER_WORD {
        WORD_MAX
    } else {
        WORD_MAX >> (BITS_PER_WORD - n)
    }
}

/// Efficient bit array using 32-bit word storage.
///
/// Provides bit-level operations with word-level performance.
/// All bit indices are 0-based.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BitArray {
    /// Storage words (32-bit)
    words: Vec<Word>,
    /// Total number of bits
    num_bits: usize,
}

impl BitArray {
    /// Create a new BitArray with `n` bits, all initialized to 0.
    ///
    /// # Examples
    ///
    /// ```
    /// use gnomics::BitArray;
    ///
    /// let ba = BitArray::new(1024);
    /// assert_eq!(ba.num_bits(), 1024);
    /// assert_eq!(ba.num_set(), 0);
    /// ```
    pub fn new(n: usize) -> Self {
        let num_words = (n + BITS_PER_WORD - 1) / BITS_PER_WORD;
        Self {
            words: vec![0; num_words],
            num_bits: n,
        }
    }

    /// Resize the BitArray to contain `n` bits.
    ///
    /// New bits are initialized to 0. If shrinking, excess bits are discarded.
    pub fn resize(&mut self, n: usize) {
        let num_words = (n + BITS_PER_WORD - 1) / BITS_PER_WORD;
        self.words.resize(num_words, 0);
        self.num_bits = n;
        self.clear_all();
    }

    /// Clear all storage and set size to 0.
    pub fn erase(&mut self) {
        self.words.clear();
        self.num_bits = 0;
    }

    // =========================================================================
    // Single Bit Operations
    // =========================================================================

    /// Set bit at position `b` to 1.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if `b >= num_bits`.
    #[inline]
    pub fn set_bit(&mut self, b: usize) {
        debug_assert!(b < self.num_bits, "bit index {} out of bounds (length: {})", b, self.num_bits);
        self.words[get_word_idx(b)] |= 1 << get_bit_idx(b);
    }

    /// Get bit at position `b` (returns 0 or 1 as u8).
    ///
    /// # Panics
    ///
    /// Panics in debug mode if `b >= num_bits`.
    #[inline]
    pub fn get_bit(&self, b: usize) -> u8 {
        debug_assert!(b < self.num_bits, "bit index {} out of bounds (length: {})", b, self.num_bits);
        ((self.words[get_word_idx(b)] >> get_bit_idx(b)) & 1) as u8
    }

    /// Clear bit at position `b` (set to 0).
    ///
    /// # Panics
    ///
    /// Panics in debug mode if `b >= num_bits`.
    #[inline]
    pub fn clear_bit(&mut self, b: usize) {
        debug_assert!(b < self.num_bits, "bit index {} out of bounds (length: {})", b, self.num_bits);
        self.words[get_word_idx(b)] &= !(1 << get_bit_idx(b));
    }

    /// Toggle bit at position `b` (0 -> 1, 1 -> 0).
    ///
    /// # Panics
    ///
    /// Panics in debug mode if `b >= num_bits`.
    #[inline]
    pub fn toggle_bit(&mut self, b: usize) {
        debug_assert!(b < self.num_bits, "bit index {} out of bounds (length: {})", b, self.num_bits);
        self.words[get_word_idx(b)] ^= 1 << get_bit_idx(b);
    }

    /// Assign bit at position `b` to given value (0 or 1).
    ///
    /// Any non-zero value is treated as 1.
    #[inline]
    pub fn assign_bit(&mut self, b: usize, val: u8) {
        if val > 0 {
            self.set_bit(b);
        } else {
            self.clear_bit(b);
        }
    }

    // =========================================================================
    // Range Operations
    // =========================================================================

    /// Set range of bits [beg, beg+len) to 1.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if beg + len > num_bits.
    pub fn set_range(&mut self, beg: usize, len: usize) {
        debug_assert!(beg + len <= self.num_bits);
        for b in beg..(beg + len) {
            self.set_bit(b);
        }
    }

    /// Clear range of bits [beg, beg+len) to 0.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if beg + len > num_bits.
    pub fn clear_range(&mut self, beg: usize, len: usize) {
        debug_assert!(beg + len <= self.num_bits);
        for b in beg..(beg + len) {
            self.clear_bit(b);
        }
    }

    /// Toggle range of bits [beg, beg+len).
    ///
    /// # Panics
    ///
    /// Panics in debug mode if beg + len > num_bits.
    pub fn toggle_range(&mut self, beg: usize, len: usize) {
        debug_assert!(beg + len <= self.num_bits);
        for b in beg..(beg + len) {
            self.toggle_bit(b);
        }
    }

    // =========================================================================
    // Bulk Operations
    // =========================================================================

    /// Set all bits to 1.
    pub fn set_all(&mut self) {
        self.words.fill(WORD_MAX);
        // Clear any bits beyond num_bits in the last word
        if self.num_bits % BITS_PER_WORD != 0 {
            let last_idx = self.words.len() - 1;
            let valid_bits = self.num_bits % BITS_PER_WORD;
            let mask = bitmask(valid_bits);
            self.words[last_idx] &= mask;
        }
    }

    /// Clear all bits to 0.
    pub fn clear_all(&mut self) {
        self.words.fill(0);
    }

    /// Toggle all bits (binary NOT operation).
    pub fn toggle_all(&mut self) {
        for word in &mut self.words {
            *word = !*word;
        }
    }

    // =========================================================================
    // Vector Operations
    // =========================================================================

    /// Set bits from vector of values (0 or 1).
    ///
    /// Clears all bits first, then sets bits where vals[i] > 0.
    pub fn set_bits(&mut self, vals: &[u8]) {
        debug_assert!(vals.len() <= self.num_bits);
        self.clear_all();
        for (i, &val) in vals.iter().enumerate() {
            if val > 0 {
                self.set_bit(i);
            }
        }
    }

    /// Set bits from vector of indices.
    ///
    /// Clears all bits first, then sets bits at indices in `idxs`.
    /// Indices >= num_bits are silently ignored.
    pub fn set_acts(&mut self, idxs: &[usize]) {
        self.clear_all();
        for &idx in idxs {
            if idx < self.num_bits {
                self.set_bit(idx);
            }
        }
    }

    /// Get all bit values as vector of 0s and 1s.
    pub fn get_bits(&self) -> Vec<u8> {
        (0..self.num_bits).map(|b| self.get_bit(b)).collect()
    }

    /// Get indices of all set bits.
    ///
    /// Returns a vector of indices where bits are 1, in ascending order.
    pub fn get_acts(&self) -> Vec<usize> {
        let mut acts = Vec::with_capacity(self.num_set());
        for (word_idx, &word) in self.words.iter().enumerate() {
            if word == 0 {
                continue;
            }
            let base = word_idx * BITS_PER_WORD;
            for bit_idx in 0..BITS_PER_WORD {
                let bit_pos = base + bit_idx;
                if bit_pos >= self.num_bits {
                    break;
                }
                if (word >> bit_idx) & 1 == 1 {
                    acts.push(bit_pos);
                }
            }
        }
        acts
    }

    // =========================================================================
    // Counting Operations
    // =========================================================================

    /// Count number of set bits (population count).
    ///
    /// Uses hardware popcount instruction for performance.
    #[inline]
    pub fn num_set(&self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// Count number of cleared bits.
    #[inline]
    pub fn num_cleared(&self) -> usize {
        self.num_bits - self.num_set()
    }

    /// Count number of similar set bits between two BitArrays.
    ///
    /// Returns count of bits that are 1 in both arrays (bitwise AND + popcount).
    ///
    /// # Panics
    ///
    /// Panics if arrays have different word counts.
    pub fn num_similar(&self, other: &BitArray) -> usize {
        assert_eq!(
            self.words.len(),
            other.words.len(),
            "BitArrays must have same word count"
        );
        self.words
            .iter()
            .zip(other.words.iter())
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
        debug_assert!(beg < self.num_bits);
        if self.num_bits == 0 {
            return None;
        }
        self.find_next_set_bit_range(beg, self.num_bits - beg)
    }

    /// Find next set bit in range [beg, beg+len), with wrapping.
    ///
    /// Returns Some(index) if found, None otherwise.
    pub fn find_next_set_bit_range(&self, beg: usize, len: usize) -> Option<usize> {
        debug_assert!(beg < self.num_bits);
        debug_assert!(len > 0 && len <= self.num_bits);

        // Calculate end position (with wrapping)
        let mut end = beg + len;
        if end > self.num_bits {
            end -= self.num_bits;
        }

        // Calculate number of words to check
        let num_words = (len / BITS_PER_WORD) + 1;

        let beg_word = get_word_idx(beg);
        let beg_bit = get_bit_idx(beg);
        let end_word = get_word_idx(end);
        let end_bit = get_bit_idx(end);

        let beg_mask = bitmask(beg_bit);
        let end_mask = bitmask(end_bit);

        // Single word case
        if num_words == 1 {
            // Check high side of beg
            let mut word = self.words[beg_word] & !beg_mask;
            if word > 0 {
                return Some(beg_word * BITS_PER_WORD + word.trailing_zeros() as usize);
            }

            // Check low side of end
            word = self.words[beg_word] & end_mask;
            if word > 0 {
                return Some(beg_word * BITS_PER_WORD + word.trailing_zeros() as usize);
            }

            return None;
        }

        // Multiple words case

        // Check high side of first word
        let mut word = self.words[beg_word] & !beg_mask;
        if word > 0 {
            return Some(beg_word * BITS_PER_WORD + word.trailing_zeros() as usize);
        }

        // Check middle words
        let mid = if beg_word == end_word {
            num_words
        } else {
            num_words - 1
        };

        for i in 1..mid {
            let j = beg_word + i;
            let w = if j < self.words.len() {
                j
            } else {
                j - self.words.len()
            };

            word = self.words[w];
            if word > 0 {
                return Some(w * BITS_PER_WORD + word.trailing_zeros() as usize);
            }
        }

        // Check low side of last word (if within bounds)
        if end_word < self.words.len() {
            word = self.words[end_word] & end_mask;
            if word > 0 {
                return Some(end_word * BITS_PER_WORD + word.trailing_zeros() as usize);
            }
        }

        None
    }

    // =========================================================================
    // Random Operations
    // =========================================================================

    /// Randomly shuffle all bits using Fisher-Yates algorithm.
    pub fn random_shuffle<R: Rng>(&mut self, rng: &mut R) {
        for i in (1..self.num_bits).rev() {
            let j = rng.gen_range(0..=i);
            let temp = self.get_bit(i);
            self.assign_bit(i, self.get_bit(j));
            self.assign_bit(j, temp);
        }
    }

    /// Randomly set exactly `num` bits to 1.
    ///
    /// Clears all bits, sets first `num` bits to 1, then shuffles.
    pub fn random_set_num<R: Rng>(&mut self, rng: &mut R, num: usize) {
        debug_assert!(num <= self.num_bits);
        self.clear_all();
        for i in 0..num {
            self.set_bit(i);
        }
        self.random_shuffle(rng);
    }

    /// Randomly set approximately `pct * num_bits` bits to 1.
    ///
    /// `pct` should be in range [0.0, 1.0].
    pub fn random_set_pct<R: Rng>(&mut self, rng: &mut R, pct: f64) {
        debug_assert!(pct >= 0.0 && pct <= 1.0);
        let num = (self.num_bits as f64 * pct) as usize;
        self.random_set_num(rng, num);
    }

    // =========================================================================
    // Information and Access
    // =========================================================================

    /// Get number of bits in array.
    #[inline]
    pub fn num_bits(&self) -> usize {
        self.num_bits
    }

    /// Get number of words in storage.
    ///
    /// CRITICAL: Used for word-level copying in Phase 2 (BlockInput::pull).
    #[inline]
    pub fn num_words(&self) -> usize {
        self.words.len()
    }

    /// Get direct read-only access to word storage.
    ///
    /// CRITICAL: Used for efficient word-level copying in Phase 2.
    #[inline]
    pub fn words(&self) -> &[Word] {
        &self.words
    }

    /// Get direct mutable access to word storage.
    ///
    /// CRITICAL: Used for efficient word-level copying in Phase 2.
    #[inline]
    pub fn words_mut(&mut self) -> &mut [Word] {
        &mut self.words
    }

    /// Estimate memory usage in bytes.
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() + self.words.capacity() * std::mem::size_of::<Word>()
    }

    /// Print bits in compact format (for debugging).
    #[allow(dead_code)]
    pub fn print_bits(&self) {
        print!("{{");
        for i in 0..self.num_bits {
            print!("{}", self.get_bit(i));
        }
        println!("}}");
    }

    /// Print active bit indices (for debugging).
    #[allow(dead_code)]
    pub fn print_acts(&self) {
        let acts = self.get_acts();
        print!("{{");
        for (i, act) in acts.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            print!("{}", act);
        }
        println!("}}");
    }
}

// =============================================================================
// Bitwise Operators
// =============================================================================

impl BitAnd for BitArray {
    type Output = BitArray;

    fn bitand(self, rhs: Self) -> Self::Output {
        &self & &rhs
    }
}

impl BitAnd for &BitArray {
    type Output = BitArray;

    fn bitand(self, rhs: Self) -> Self::Output {
        assert_eq!(self.num_bits, rhs.num_bits, "BitArrays must have same size");
        let words: Vec<Word> = self
            .words
            .iter()
            .zip(rhs.words.iter())
            .map(|(a, b)| a & b)
            .collect();
        BitArray {
            words,
            num_bits: self.num_bits,
        }
    }
}

impl BitOr for BitArray {
    type Output = BitArray;

    fn bitor(self, rhs: Self) -> Self::Output {
        &self | &rhs
    }
}

impl BitOr for &BitArray {
    type Output = BitArray;

    fn bitor(self, rhs: Self) -> Self::Output {
        assert_eq!(self.num_bits, rhs.num_bits, "BitArrays must have same size");
        let words: Vec<Word> = self
            .words
            .iter()
            .zip(rhs.words.iter())
            .map(|(a, b)| a | b)
            .collect();
        BitArray {
            words,
            num_bits: self.num_bits,
        }
    }
}

impl BitXor for BitArray {
    type Output = BitArray;

    fn bitxor(self, rhs: Self) -> Self::Output {
        &self ^ &rhs
    }
}

impl BitXor for &BitArray {
    type Output = BitArray;

    fn bitxor(self, rhs: Self) -> Self::Output {
        assert_eq!(self.num_bits, rhs.num_bits, "BitArrays must have same size");
        let words: Vec<Word> = self
            .words
            .iter()
            .zip(rhs.words.iter())
            .map(|(a, b)| a ^ b)
            .collect();
        BitArray {
            words,
            num_bits: self.num_bits,
        }
    }
}

impl Not for BitArray {
    type Output = BitArray;

    fn not(self) -> Self::Output {
        !&self
    }
}

impl Not for &BitArray {
    type Output = BitArray;

    fn not(self) -> Self::Output {
        let words: Vec<Word> = self.words.iter().map(|w| !w).collect();
        BitArray {
            words,
            num_bits: self.num_bits,
        }
    }
}

// =============================================================================
// Comparison Operators
// =============================================================================

impl PartialEq for BitArray {
    /// Compare BitArrays using word-level memcmp.
    ///
    /// CRITICAL: Used for change tracking in BlockOutput::store() (Phase 2).
    /// Must be fast - uses slice comparison which compiles to memcmp.
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.num_bits == other.num_bits && self.words == other.words
    }
}

impl Eq for BitArray {}

// =============================================================================
// Helper Functions
// =============================================================================

/// Fast word-level copy from src to dst BitArray.
///
/// CRITICAL: Used by BlockInput::pull() in Phase 2 for lazy copying.
/// This function enables efficient concatenation of child outputs.
///
/// # Arguments
///
/// * `dst` - Destination BitArray
/// * `src` - Source BitArray
/// * `dst_word_offset` - Word offset in destination
/// * `src_word_offset` - Word offset in source
/// * `num_words` - Number of words to copy
///
/// # Performance
///
/// Compiles to memcpy for optimal performance (~60ns for 1024 bits).
#[inline(always)]
pub fn bitarray_copy_words(
    dst: &mut BitArray,
    src: &BitArray,
    dst_word_offset: usize,
    src_word_offset: usize,
    num_words: usize,
) {
    let dst_start = dst_word_offset;
    let dst_end = dst_start + num_words;
    let src_start = src_word_offset;
    let src_end = src_start + num_words;

    debug_assert!(dst_end <= dst.words.len(), "dst word overflow");
    debug_assert!(src_end <= src.words.len(), "src word overflow");

    dst.words[dst_start..dst_end].copy_from_slice(&src.words[src_start..src_end]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_new() {
        let ba = BitArray::new(1024);
        assert_eq!(ba.num_bits(), 1024);
        assert_eq!(ba.num_words(), 32);
        assert_eq!(ba.num_set(), 0);
    }

    #[test]
    fn test_set_get_bit() {
        let mut ba = BitArray::new(32);
        assert_eq!(ba.get_bit(5), 0);
        ba.set_bit(5);
        assert_eq!(ba.get_bit(5), 1);
        ba.clear_bit(5);
        assert_eq!(ba.get_bit(5), 0);
    }

    #[test]
    fn test_toggle_bit() {
        let mut ba = BitArray::new(32);
        ba.toggle_bit(7);
        assert_eq!(ba.get_bit(7), 1);
        ba.toggle_bit(7);
        assert_eq!(ba.get_bit(7), 0);
    }

    #[test]
    fn test_assign_bit() {
        let mut ba = BitArray::new(32);
        ba.assign_bit(3, 1);
        assert_eq!(ba.get_bit(3), 1);
        ba.assign_bit(3, 0);
        assert_eq!(ba.get_bit(3), 0);
    }

    #[test]
    fn test_range_operations() {
        let mut ba = BitArray::new(32);
        ba.set_range(2, 8);
        assert_eq!(ba.num_set(), 8);
        assert_eq!(ba.get_acts(), vec![2, 3, 4, 5, 6, 7, 8, 9]);

        ba.clear_range(4, 4);
        assert_eq!(ba.num_set(), 4);
        assert_eq!(ba.get_acts(), vec![2, 3, 8, 9]);

        ba.toggle_range(2, 8);
        assert_eq!(ba.get_acts(), vec![4, 5, 6, 7]);
    }

    #[test]
    fn test_bulk_operations() {
        let mut ba = BitArray::new(32);
        ba.set_all();
        assert_eq!(ba.num_set(), 32);

        ba.clear_all();
        assert_eq!(ba.num_set(), 0);

        ba.set_bit(0);
        ba.set_bit(31);
        ba.toggle_all();
        assert_eq!(ba.num_set(), 30);
    }

    #[test]
    fn test_set_acts() {
        let mut ba = BitArray::new(32);
        ba.set_acts(&[2, 4, 6, 8]);
        assert_eq!(ba.num_set(), 4);
        assert_eq!(ba.get_acts(), vec![2, 4, 6, 8]);
    }

    #[test]
    fn test_set_bits() {
        let mut ba = BitArray::new(8);
        ba.set_bits(&[0, 1, 0, 1, 0, 1, 0, 1]);
        assert_eq!(ba.get_acts(), vec![1, 3, 5, 7]);
    }

    #[test]
    fn test_get_bits() {
        let mut ba = BitArray::new(8);
        ba.set_acts(&[1, 3, 5, 7]);
        assert_eq!(ba.get_bits(), vec![0, 1, 0, 1, 0, 1, 0, 1]);
    }

    #[test]
    fn test_num_similar() {
        let mut ba0 = BitArray::new(32);
        let mut ba1 = BitArray::new(32);

        ba0.set_range(4, 8);
        ba1.set_range(6, 10);

        let similar = ba0.num_similar(&ba1);
        assert_eq!(similar, 6); // bits 6-11 overlap
    }

    #[test]
    fn test_find_next_set_bit() {
        let mut ba = BitArray::new(32);
        ba.set_range(4, 8);

        assert_eq!(ba.find_next_set_bit(0), Some(4));
        assert_eq!(ba.find_next_set_bit(5), Some(5));
        assert_eq!(ba.find_next_set_bit(12), None);
    }

    #[test]
    fn test_random_operations() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let mut ba = BitArray::new(1024);

        ba.random_set_num(&mut rng, 100);
        assert_eq!(ba.num_set(), 100);

        ba.random_set_pct(&mut rng, 0.1);
        assert!(ba.num_set() >= 95 && ba.num_set() <= 105);
    }

    #[test]
    fn test_bitwise_and() {
        let mut ba0 = BitArray::new(32);
        let mut ba1 = BitArray::new(32);

        ba0.set_bit(2);
        ba0.set_bit(3);
        ba1.set_bit(1);
        ba1.set_bit(3);

        let result = &ba0 & &ba1;
        assert_eq!(result.num_set(), 1);
        assert_eq!(result.get_acts(), vec![3]);
    }

    #[test]
    fn test_bitwise_or() {
        let mut ba0 = BitArray::new(32);
        let mut ba1 = BitArray::new(32);

        ba0.set_bit(2);
        ba0.set_bit(3);
        ba1.set_bit(1);
        ba1.set_bit(3);

        let result = &ba0 | &ba1;
        assert_eq!(result.num_set(), 3);
        assert_eq!(result.get_acts(), vec![1, 2, 3]);
    }

    #[test]
    fn test_bitwise_xor() {
        let mut ba0 = BitArray::new(32);
        let mut ba1 = BitArray::new(32);

        ba0.set_bit(2);
        ba0.set_bit(3);
        ba1.set_bit(1);
        ba1.set_bit(3);

        let result = &ba0 ^ &ba1;
        assert_eq!(result.num_set(), 2);
        assert_eq!(result.get_acts(), vec![1, 2]);
    }

    #[test]
    fn test_bitwise_not() {
        let mut ba = BitArray::new(32);
        ba.set_bit(2);
        ba.set_bit(3);

        let result = !&ba;
        assert_eq!(result.num_set(), 30);
    }

    #[test]
    fn test_equality() {
        let mut ba0 = BitArray::new(32);
        let mut ba1 = BitArray::new(32);

        ba0.set_bit(5);
        ba1.set_bit(5);

        assert_eq!(ba0, ba1);

        ba1.set_bit(10);
        assert_ne!(ba0, ba1);
    }

    #[test]
    fn test_bitarray_copy_words() {
        let mut src = BitArray::new(128);
        let mut dst = BitArray::new(256);

        src.set_range(0, 64);
        bitarray_copy_words(&mut dst, &src, 2, 0, 2);

        // Check that words 2-3 in dst match words 0-1 in src
        assert_eq!(dst.words[2], src.words[0]);
        assert_eq!(dst.words[3], src.words[1]);
    }

    #[test]
    fn test_resize() {
        let mut ba = BitArray::new(32);
        ba.set_all();
        assert_eq!(ba.num_set(), 32);

        ba.resize(64);
        assert_eq!(ba.num_bits(), 64);
        assert_eq!(ba.num_set(), 0); // resize clears
    }

    #[test]
    fn test_erase() {
        let mut ba = BitArray::new(32);
        ba.set_all();
        ba.erase();
        assert_eq!(ba.num_bits(), 0);
        assert_eq!(ba.num_words(), 0);
    }

    #[test]
    fn test_memory_usage() {
        let ba = BitArray::new(1024);
        let usage = ba.memory_usage();
        assert!(usage >= 128); // At least 32 words * 4 bytes
    }
}
