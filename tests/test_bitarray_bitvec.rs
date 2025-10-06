//! Tests for BitArrayBitvec prototype implementation.
//!
//! These tests validate:
//! - API compatibility with custom BitArray
//! - Correctness of critical operations
//! - Word-level access and copying
//! - Operator implementations

use gnomics::{bitarray_copy_words_bitvec, BitArrayBitvec};
use rand::SeedableRng;

// =============================================================================
// Basic Operations
// =============================================================================

#[test]
fn test_new() {
    let ba = BitArrayBitvec::new(1024);
    assert_eq!(ba.num_bits(), 1024);
    assert_eq!(ba.num_set(), 0);
    assert_eq!(ba.num_cleared(), 1024);
}

#[test]
fn test_resize() {
    let mut ba = BitArrayBitvec::new(1024);
    ba.set_bit(10);
    ba.set_bit(100);

    ba.resize(2048);
    assert_eq!(ba.num_bits(), 2048);
    assert_eq!(ba.get_bit(10), 1);
    assert_eq!(ba.get_bit(100), 1);

    ba.resize(512);
    assert_eq!(ba.num_bits(), 512);
    assert_eq!(ba.get_bit(10), 1);
    assert_eq!(ba.get_bit(100), 1);
}

#[test]
fn test_erase() {
    let mut ba = BitArrayBitvec::new(1024);
    ba.set_bit(10);
    ba.erase();
    assert_eq!(ba.num_bits(), 0);
}

// =============================================================================
// Single Bit Operations
// =============================================================================

#[test]
fn test_set_get_bit() {
    let mut ba = BitArrayBitvec::new(1024);
    ba.set_bit(5);
    ba.set_bit(100);
    ba.set_bit(500);

    assert_eq!(ba.get_bit(5), 1);
    assert_eq!(ba.get_bit(100), 1);
    assert_eq!(ba.get_bit(500), 1);
    assert_eq!(ba.get_bit(10), 0);
    assert_eq!(ba.num_set(), 3);
}

#[test]
fn test_clear_bit() {
    let mut ba = BitArrayBitvec::new(1024);
    ba.set_bit(5);
    ba.set_bit(10);
    ba.set_bit(15);

    ba.clear_bit(10);
    assert_eq!(ba.get_bit(5), 1);
    assert_eq!(ba.get_bit(10), 0);
    assert_eq!(ba.get_bit(15), 1);
    assert_eq!(ba.num_set(), 2);
}

#[test]
fn test_toggle_bit() {
    let mut ba = BitArrayBitvec::new(1024);
    ba.toggle_bit(5);
    assert_eq!(ba.get_bit(5), 1);

    ba.toggle_bit(5);
    assert_eq!(ba.get_bit(5), 0);
}

#[test]
fn test_assign_bit() {
    let mut ba = BitArrayBitvec::new(1024);
    ba.assign_bit(5, 1);
    assert_eq!(ba.get_bit(5), 1);

    ba.assign_bit(5, 0);
    assert_eq!(ba.get_bit(5), 0);

    ba.assign_bit(10, 42); // Any non-zero is treated as 1
    assert_eq!(ba.get_bit(10), 1);
}

// =============================================================================
// Bulk Operations
// =============================================================================

#[test]
fn test_set_all() {
    let mut ba = BitArrayBitvec::new(1024);
    ba.set_all();
    assert_eq!(ba.num_set(), 1024);
}

#[test]
fn test_clear_all() {
    let mut ba = BitArrayBitvec::new(1024);
    ba.set_all();
    ba.clear_all();
    assert_eq!(ba.num_set(), 0);
}

#[test]
fn test_toggle_all() {
    let mut ba = BitArrayBitvec::new(1024);
    ba.set_bit(5);
    ba.toggle_all();
    assert_eq!(ba.get_bit(5), 0);
    assert_eq!(ba.get_bit(10), 1);
    assert_eq!(ba.num_set(), 1023);
}

#[test]
fn test_set_range() {
    let mut ba = BitArrayBitvec::new(1024);
    ba.set_range(10, 5);
    assert_eq!(ba.get_acts(), vec![10, 11, 12, 13, 14]);
}

#[test]
fn test_clear_range() {
    let mut ba = BitArrayBitvec::new(1024);
    ba.set_all();
    ba.clear_range(10, 5);
    assert_eq!(ba.num_set(), 1024 - 5);
}

// =============================================================================
// Vector Operations (CRITICAL)
// =============================================================================

#[test]
fn test_set_acts_get_acts() {
    let mut ba = BitArrayBitvec::new(1024);
    let indices = vec![5, 10, 15, 100, 500, 999];
    ba.set_acts(&indices);

    assert_eq!(ba.get_acts(), indices);
    assert_eq!(ba.num_set(), 6);
}

#[test]
fn test_set_acts_empty() {
    let mut ba = BitArrayBitvec::new(1024);
    ba.set_bit(10);
    ba.set_acts(&[]);
    assert_eq!(ba.num_set(), 0);
}

#[test]
fn test_set_acts_out_of_bounds() {
    let mut ba = BitArrayBitvec::new(1024);
    ba.set_acts(&[5, 10, 1500]); // 1500 is out of bounds
    assert_eq!(ba.get_acts(), vec![5, 10]);
}

#[test]
fn test_get_bits() {
    let mut ba = BitArrayBitvec::new(8);
    ba.set_bit(1);
    ba.set_bit(3);
    ba.set_bit(7);

    let bits = ba.get_bits();
    assert_eq!(bits, vec![0, 1, 0, 1, 0, 0, 0, 1]);
}

// =============================================================================
// Counting Operations
// =============================================================================

#[test]
fn test_num_set_cleared() {
    let mut ba = BitArrayBitvec::new(1024);
    ba.set_acts(&[5, 10, 15]);

    assert_eq!(ba.num_set(), 3);
    assert_eq!(ba.num_cleared(), 1021);
}

#[test]
fn test_num_similar() {
    let mut ba1 = BitArrayBitvec::new(1024);
    let mut ba2 = BitArrayBitvec::new(1024);

    ba1.set_acts(&[5, 10, 15, 20, 25]);
    ba2.set_acts(&[10, 15, 20, 30, 35]);

    assert_eq!(ba1.num_similar(&ba2), 3); // 10, 15, 20
}

#[test]
fn test_num_similar_no_overlap() {
    let mut ba1 = BitArrayBitvec::new(1024);
    let mut ba2 = BitArrayBitvec::new(1024);

    ba1.set_acts(&[5, 10]);
    ba2.set_acts(&[100, 200]);

    assert_eq!(ba1.num_similar(&ba2), 0);
}

#[test]
fn test_num_similar_full_overlap() {
    let mut ba1 = BitArrayBitvec::new(1024);
    let mut ba2 = BitArrayBitvec::new(1024);

    ba1.set_acts(&[5, 10, 15]);
    ba2.set_acts(&[5, 10, 15]);

    assert_eq!(ba1.num_similar(&ba2), 3);
}

// =============================================================================
// Search Operations
// =============================================================================

#[test]
fn test_find_next_set_bit() {
    let mut ba = BitArrayBitvec::new(1024);
    ba.set_bit(5);
    ba.set_bit(100);
    ba.set_bit(500);

    assert_eq!(ba.find_next_set_bit(0), Some(5));
    assert_eq!(ba.find_next_set_bit(6), Some(100));
    assert_eq!(ba.find_next_set_bit(101), Some(500));
    assert_eq!(ba.find_next_set_bit(501), Some(5)); // Wraps around
}

#[test]
fn test_find_next_set_bit_none() {
    let ba = BitArrayBitvec::new(1024);
    assert_eq!(ba.find_next_set_bit(0), None);
}

// =============================================================================
// Random Operations
// =============================================================================

#[test]
fn test_random_set_num() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let mut ba = BitArrayBitvec::new(1024);

    ba.random_set_num(&mut rng, 100);
    assert_eq!(ba.num_set(), 100);
}

#[test]
fn test_random_set_num_exceeds_size() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let mut ba = BitArrayBitvec::new(100);

    ba.random_set_num(&mut rng, 150); // More than available bits
    assert_eq!(ba.num_set(), 100);
}

#[test]
fn test_random_set_pct() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let mut ba = BitArrayBitvec::new(1024);

    ba.random_set_pct(&mut rng, 0.1);
    // Allow ±10% tolerance for randomness
    assert!(ba.num_set() >= 92 && ba.num_set() <= 112);
}

#[test]
fn test_random_shuffle() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let mut ba = BitArrayBitvec::new(1024);

    ba.set_acts(&[5, 10, 15, 20, 25]);
    let original_count = ba.num_set();

    ba.random_shuffle(&mut rng);

    // Count should remain the same
    assert_eq!(ba.num_set(), original_count);

    // Positions likely changed (with very high probability)
    // Note: With seed 42, this should be deterministic
}

// =============================================================================
// Word-Level Access (CRITICAL for Phase 2)
// =============================================================================

#[test]
fn test_num_words() {
    let ba = BitArrayBitvec::new(1024);
    assert_eq!(ba.num_words(), 32); // 1024 / 32

    let ba = BitArrayBitvec::new(128);
    assert_eq!(ba.num_words(), 4); // 128 / 32

    let ba = BitArrayBitvec::new(100);
    assert_eq!(ba.num_words(), 4); // Ceiling: (100 + 31) / 32
}

#[test]
fn test_words_access() {
    let mut ba = BitArrayBitvec::new(1024);
    ba.set_acts(&[0, 1, 2]); // First word

    let words = ba.words();
    assert_eq!(words.len(), 32);
    assert_eq!(words[0], 0b111); // Bits 0, 1, 2 set
}

#[test]
fn test_words_mut_access() {
    let mut ba = BitArrayBitvec::new(1024);

    {
        let words = ba.words_mut();
        words[0] = 0b1010; // Set bits 1 and 3
    }

    assert_eq!(ba.get_bit(1), 1);
    assert_eq!(ba.get_bit(3), 1);
    assert_eq!(ba.get_bit(0), 0);
    assert_eq!(ba.get_bit(2), 0);
}

#[test]
fn test_bitarray_copy_words_basic() {
    let mut src = BitArrayBitvec::new(1024);
    let mut dst = BitArrayBitvec::new(1024);

    src.set_acts(&[5, 100, 500]);

    // Copy all words from src to dst
    bitarray_copy_words_bitvec(&mut dst, &src, 0, 0, src.num_words());

    assert_eq!(dst.get_acts(), vec![5, 100, 500]);
    assert_eq!(dst.num_set(), 3);
}

#[test]
fn test_bitarray_copy_words_partial() {
    let mut src = BitArrayBitvec::new(128);
    let mut dst = BitArrayBitvec::new(256);

    src.set_range(0, 64); // Set first 64 bits (2 words)

    // Copy first 2 words from src to words 2-3 in dst
    bitarray_copy_words_bitvec(&mut dst, &src, 2, 0, 2);

    // Check that bits 64-127 in dst are set (words 2-3)
    assert_eq!(dst.get_bit(64), 1);
    assert_eq!(dst.get_bit(127), 1);
    assert_eq!(dst.get_bit(0), 0); // Not copied
    assert_eq!(dst.get_bit(128), 0); // Not copied
}

#[test]
fn test_bitarray_copy_words_offset() {
    let mut src = BitArrayBitvec::new(1024);
    let mut dst = BitArrayBitvec::new(1024);

    // Set bits in second word of src
    src.set_range(32, 32); // Bits 32-63

    // Copy second word of src to first word of dst
    bitarray_copy_words_bitvec(&mut dst, &src, 0, 1, 1);

    // Bits 0-31 in dst should now match bits 32-63 in src
    assert_eq!(dst.get_bit(0), 1);
    assert_eq!(dst.get_bit(31), 1);
}

// =============================================================================
// Operator Tests
// =============================================================================

#[test]
fn test_bitwise_and() {
    let mut ba1 = BitArrayBitvec::new(32);
    let mut ba2 = BitArrayBitvec::new(32);

    ba1.set_acts(&[0, 5, 10]);
    ba2.set_acts(&[5, 10, 15]);

    let result = &ba1 & &ba2;
    assert_eq!(result.get_acts(), vec![5, 10]);
}

#[test]
fn test_bitwise_or() {
    let mut ba1 = BitArrayBitvec::new(32);
    let mut ba2 = BitArrayBitvec::new(32);

    ba1.set_acts(&[0, 5, 10]);
    ba2.set_acts(&[5, 10, 15]);

    let result = &ba1 | &ba2;
    assert_eq!(result.get_acts(), vec![0, 5, 10, 15]);
}

#[test]
fn test_bitwise_xor() {
    let mut ba1 = BitArrayBitvec::new(32);
    let mut ba2 = BitArrayBitvec::new(32);

    ba1.set_acts(&[0, 5, 10]);
    ba2.set_acts(&[5, 10, 15]);

    let result = &ba1 ^ &ba2;
    assert_eq!(result.get_acts(), vec![0, 15]);
}

#[test]
fn test_bitwise_not() {
    let mut ba = BitArrayBitvec::new(32);
    ba.set_acts(&[0, 5]);

    let result = !&ba;
    assert_eq!(result.num_set(), 30);
    assert_eq!(result.get_bit(0), 0);
    assert_eq!(result.get_bit(5), 0);
    assert_eq!(result.get_bit(1), 1);
}

// =============================================================================
// Equality Tests (CRITICAL for change tracking)
// =============================================================================

#[test]
fn test_equality_same() {
    let mut ba1 = BitArrayBitvec::new(1024);
    let mut ba2 = BitArrayBitvec::new(1024);

    ba1.set_acts(&[5, 10, 15]);
    ba2.set_acts(&[5, 10, 15]);

    assert_eq!(ba1, ba2);
}

#[test]
fn test_equality_different() {
    let mut ba1 = BitArrayBitvec::new(1024);
    let mut ba2 = BitArrayBitvec::new(1024);

    ba1.set_acts(&[5, 10]);
    ba2.set_acts(&[5, 10, 15]);

    assert_ne!(ba1, ba2);
}

#[test]
fn test_equality_empty() {
    let ba1 = BitArrayBitvec::new(1024);
    let ba2 = BitArrayBitvec::new(1024);

    assert_eq!(ba1, ba2);
}

#[test]
fn test_equality_full() {
    let mut ba1 = BitArrayBitvec::new(1024);
    let mut ba2 = BitArrayBitvec::new(1024);

    ba1.set_all();
    ba2.set_all();

    assert_eq!(ba1, ba2);
}

// =============================================================================
// Memory Usage
// =============================================================================

#[test]
fn test_memory_usage() {
    let ba = BitArrayBitvec::new(1024);
    let mem = ba.memory_usage();

    // Should be at least size of struct + word storage
    // 1024 bits = 32 words = 128 bytes minimum
    assert!(mem >= 128);
}
