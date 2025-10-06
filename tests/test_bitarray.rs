//! Comprehensive tests for BitArray implementation.
//!
//! These tests match the behavior demonstrated in the C++ test file
//! (tests/cpp/test_bitarray.cpp) and add additional property-based tests.

use gnomics::{bitarray_copy_words, BitArray};
use proptest::prelude::*;
use rand::SeedableRng;

// =============================================================================
// Basic Construction and Operations
// =============================================================================

#[test]
fn test_construction() {
    let ba = BitArray::new(1024);
    assert_eq!(ba.num_bits(), 1024);
    assert_eq!(ba.num_words(), 32);
    assert_eq!(ba.num_set(), 0);
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

// =============================================================================
// Single Bit Operations
// =============================================================================

#[test]
fn test_set_bit() {
    let mut ba = BitArray::new(1024);
    ba.set_bit(4);
    assert_eq!(ba.get_bit(4), 1);
    assert_eq!(ba.num_set(), 1);
}

#[test]
fn test_get_bit() {
    let mut ba = BitArray::new(1024);
    ba.set_bit(4);
    assert_eq!(ba.get_bit(4), 1);
    assert_eq!(ba.get_bit(5), 0);
}

#[test]
fn test_clear_bit() {
    let mut ba = BitArray::new(1024);
    ba.set_bit(4);
    ba.clear_bit(4);
    assert_eq!(ba.get_bit(4), 0);
}

#[test]
fn test_toggle_bit() {
    let mut ba = BitArray::new(1024);
    ba.toggle_bit(7);
    assert_eq!(ba.get_bit(7), 1);
    ba.toggle_bit(7);
    assert_eq!(ba.get_bit(7), 0);
}

#[test]
fn test_assign_bit() {
    let mut ba = BitArray::new(1024);
    ba.assign_bit(7, 1);
    assert_eq!(ba.get_bit(7), 1);
    ba.assign_bit(7, 0);
    assert_eq!(ba.get_bit(7), 0);
}

// =============================================================================
// Range Operations
// =============================================================================

#[test]
fn test_set_range() {
    let mut ba = BitArray::new(1024);
    ba.set_range(2, 8);
    assert_eq!(ba.num_set(), 8);
    let acts = ba.get_acts();
    assert_eq!(acts, vec![2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn test_toggle_range() {
    let mut ba = BitArray::new(1024);
    ba.set_range(2, 8); // Sets bits 2-9
    ba.toggle_range(4, 8); // Toggles bits 4-11
    let acts = ba.get_acts();
    // After toggle: 2,3 stay set, 4-9 become unset, 10-11 become set
    assert_eq!(acts, vec![2, 3, 10, 11]);
}

#[test]
fn test_clear_range() {
    let mut ba = BitArray::new(1024);
    ba.set_range(2, 8);
    ba.clear_range(2, 10);
    assert_eq!(ba.num_set(), 0);
}

// =============================================================================
// Bulk Operations
// =============================================================================

#[test]
fn test_set_all() {
    let mut ba = BitArray::new(1024);
    ba.set_all();
    assert_eq!(ba.num_set(), 1024);
}

#[test]
fn test_clear_all() {
    let mut ba = BitArray::new(1024);
    ba.set_all();
    ba.clear_all();
    assert_eq!(ba.num_set(), 0);
}

#[test]
fn test_toggle_all() {
    let mut ba = BitArray::new(1024);
    ba.set_all();
    ba.toggle_all();
    assert_eq!(ba.num_set(), 0);
}

// =============================================================================
// Vector Operations
// =============================================================================

#[test]
fn test_set_bits() {
    let mut ba = BitArray::new(1024);
    let vals = vec![0, 1, 0, 1, 0, 1, 0, 1];
    ba.set_bits(&vals);
    let acts = ba.get_acts();
    assert_eq!(acts, vec![1, 3, 5, 7]);
}

#[test]
fn test_set_acts() {
    let mut ba = BitArray::new(1024);
    ba.set_acts(&[2, 4, 6, 8]);
    assert_eq!(ba.num_set(), 4);
    assert_eq!(ba.get_acts(), vec![2, 4, 6, 8]);
}

#[test]
fn test_get_bits() {
    let mut ba = BitArray::new(8);
    ba.set_acts(&[1, 3, 5, 7]);
    assert_eq!(ba.get_bits(), vec![0, 1, 0, 1, 0, 1, 0, 1]);
}

#[test]
fn test_get_acts() {
    let mut ba = BitArray::new(1024);
    ba.set_range(4, 8);
    let acts = ba.get_acts();
    assert_eq!(acts.len(), 8);
    assert_eq!(acts, vec![4, 5, 6, 7, 8, 9, 10, 11]);
}

// =============================================================================
// Counting Operations
// =============================================================================

#[test]
fn test_num_set() {
    let mut ba = BitArray::new(1024);
    ba.set_range(4, 8);
    assert_eq!(ba.num_set(), 8);
}

#[test]
fn test_num_cleared() {
    let mut ba = BitArray::new(1024);
    ba.set_range(4, 8);
    assert_eq!(ba.num_cleared(), 1016);
}

#[test]
fn test_num_similar() {
    let mut ba0 = BitArray::new(1024);
    let mut ba2 = BitArray::new(1024);

    ba0.set_range(4, 8);
    ba2.set_range(6, 10);

    let num_similar = ba2.num_similar(&ba0);
    assert_eq!(num_similar, 6); // Bits 6-11 overlap
}

// =============================================================================
// Search Operations
// =============================================================================

#[test]
fn test_find_next_set_bit() {
    let mut ba = BitArray::new(1024);
    ba.set_range(4, 8);

    assert_eq!(ba.find_next_set_bit(0), Some(4));
    assert_eq!(ba.find_next_set_bit(5), Some(5));
    assert_eq!(ba.find_next_set_bit(12), None);
}

#[test]
fn test_find_next_set_bit_range() {
    let mut ba = BitArray::new(1024);
    ba.set_range(4, 8);

    let result = ba.find_next_set_bit_range(6, 18);
    assert!(result.is_some());
    let next_bit = result.unwrap();
    assert!(next_bit >= 4 && next_bit < 12);
}

// =============================================================================
// Random Operations
// =============================================================================

#[test]
fn test_random_shuffle() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let mut ba = BitArray::new(1024);
    ba.set_range(0, 100);

    let acts_before = ba.get_acts();
    ba.random_shuffle(&mut rng);
    let acts_after = ba.get_acts();

    // Should have same number of bits
    assert_eq!(acts_before.len(), acts_after.len());
    // But different positions (with very high probability)
    assert_ne!(acts_before, acts_after);
}

#[test]
fn test_random_set_num() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let mut ba = BitArray::new(1024);

    ba.random_set_num(&mut rng, 100);
    assert_eq!(ba.num_set(), 100);
}

#[test]
fn test_random_set_pct() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let mut ba = BitArray::new(1024);

    ba.random_set_pct(&mut rng, 0.1);
    let num_set = ba.num_set();
    // Should be approximately 102 bits (10% of 1024)
    assert!(num_set >= 90 && num_set <= 114, "num_set: {}", num_set);
}

#[test]
fn test_random_deterministic() {
    // Same seed should produce same results
    let mut rng1 = rand::rngs::StdRng::seed_from_u64(42);
    let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);

    let mut ba1 = BitArray::new(1024);
    let mut ba2 = BitArray::new(1024);

    ba1.random_set_num(&mut rng1, 100);
    ba2.random_set_num(&mut rng2, 100);

    assert_eq!(ba1.get_acts(), ba2.get_acts());
}

// =============================================================================
// Bitwise Operators
// =============================================================================

#[test]
fn test_bitwise_not() {
    let mut ba0 = BitArray::new(1024);
    ba0.set_bit(2);
    ba0.set_bit(3);

    let ba2 = !&ba0;
    assert_eq!(ba2.num_set(), 1022);
}

#[test]
fn test_bitwise_and() {
    let mut ba0 = BitArray::new(1024);
    let mut ba1 = BitArray::new(1024);

    ba0.set_bit(2);
    ba0.set_bit(3);
    ba1.set_bit(1);
    ba1.set_bit(3);

    let ba2 = &ba0 & &ba1;
    assert_eq!(ba2.num_set(), 1);
    assert_eq!(ba2.get_acts(), vec![3]);
}

#[test]
fn test_bitwise_or() {
    let mut ba0 = BitArray::new(1024);
    let mut ba1 = BitArray::new(1024);

    ba0.set_bit(2);
    ba0.set_bit(3);
    ba1.set_bit(1);
    ba1.set_bit(3);

    let ba2 = &ba0 | &ba1;
    assert_eq!(ba2.num_set(), 3);
    assert_eq!(ba2.get_acts(), vec![1, 2, 3]);
}

#[test]
fn test_bitwise_xor() {
    let mut ba0 = BitArray::new(1024);
    let mut ba1 = BitArray::new(1024);

    ba0.set_bit(2);
    ba0.set_bit(3);
    ba1.set_bit(1);
    ba1.set_bit(3);

    let ba2 = &ba0 ^ &ba1;
    assert_eq!(ba2.num_set(), 2);
    assert_eq!(ba2.get_acts(), vec![1, 2]);
}

// =============================================================================
// Comparison Operators
// =============================================================================

#[test]
fn test_equality() {
    let mut ba0 = BitArray::new(1024);
    let mut ba1 = BitArray::new(1024);

    ba0.set_bit(5);
    ba1.set_bit(5);

    assert_eq!(ba0, ba1);

    ba1.set_bit(10);
    assert_ne!(ba0, ba1);
}

#[test]
fn test_equality_performance() {
    // Test that equality uses fast word-level comparison
    let mut ba1 = BitArray::new(10000);
    let mut ba2 = BitArray::new(10000);

    ba1.random_set_pct(&mut rand::rngs::StdRng::seed_from_u64(0), 0.2);
    ba2 = ba1.clone();

    // Should be equal
    assert_eq!(ba1, ba2);

    // Change one bit
    ba2.toggle_bit(5000);
    assert_ne!(ba1, ba2);
}

// =============================================================================
// Word-Level Operations
// =============================================================================

#[test]
fn test_word_access() {
    let mut ba = BitArray::new(128);
    ba.set_range(0, 64);

    let words = ba.words();
    assert_eq!(words.len(), 4);
    assert_eq!(words[0], 0xFFFFFFFF);
    assert_eq!(words[1], 0xFFFFFFFF);
}

#[test]
fn test_bitarray_copy_words() {
    let mut src = BitArray::new(128);
    let mut dst = BitArray::new(256);

    src.set_range(0, 64);
    bitarray_copy_words(&mut dst, &src, 2, 0, 2);

    // Check that words 2-3 in dst match words 0-1 in src
    assert_eq!(dst.words()[2], src.words()[0]);
    assert_eq!(dst.words()[3], src.words()[1]);
}

#[test]
fn test_bitarray_copy_words_multiple() {
    let mut src1 = BitArray::new(64);
    let mut src2 = BitArray::new(64);
    let mut dst = BitArray::new(256);

    src1.set_range(0, 32);
    src2.set_range(0, 32);

    // Copy src1 to words 0-1
    bitarray_copy_words(&mut dst, &src1, 0, 0, 2);

    // Copy src2 to words 2-3
    bitarray_copy_words(&mut dst, &src2, 2, 0, 2);

    // Verify
    assert_eq!(dst.words()[0], src1.words()[0]);
    assert_eq!(dst.words()[1], src1.words()[1]);
    assert_eq!(dst.words()[2], src2.words()[0]);
    assert_eq!(dst.words()[3], src2.words()[1]);
}

// =============================================================================
// Memory and Information
// =============================================================================

#[test]
fn test_memory_usage() {
    let ba = BitArray::new(1024);
    let usage = ba.memory_usage();
    assert!(usage >= 128); // At least 32 words * 4 bytes
}

#[test]
fn test_num_words() {
    let ba = BitArray::new(1024);
    assert_eq!(ba.num_words(), 32);

    let ba = BitArray::new(1000);
    assert_eq!(ba.num_words(), 32); // Rounds up

    let ba = BitArray::new(32);
    assert_eq!(ba.num_words(), 1);
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_empty_bitarray() {
    let ba = BitArray::new(0);
    assert_eq!(ba.num_bits(), 0);
    assert_eq!(ba.num_words(), 0);
    assert_eq!(ba.num_set(), 0);
}

#[test]
fn test_single_bit() {
    let mut ba = BitArray::new(1);
    assert_eq!(ba.num_set(), 0);
    ba.set_bit(0);
    assert_eq!(ba.num_set(), 1);
}

#[test]
fn test_cross_word_boundary() {
    let mut ba = BitArray::new(128);
    ba.set_bit(31); // Last bit of word 0
    ba.set_bit(32); // First bit of word 1
    ba.set_bit(63); // Last bit of word 1
    ba.set_bit(64); // First bit of word 2

    assert_eq!(ba.num_set(), 4);
    assert_eq!(ba.get_acts(), vec![31, 32, 63, 64]);
}

// =============================================================================
// Property-Based Tests
// =============================================================================

proptest! {
    #[test]
    fn prop_set_get_consistency(bits in prop::collection::vec(any::<bool>(), 1..1000)) {
        let mut ba = BitArray::new(bits.len());
        for (i, &b) in bits.iter().enumerate() {
            if b {
                ba.set_bit(i);
            }
        }

        for (i, &b) in bits.iter().enumerate() {
            prop_assert_eq!(ba.get_bit(i), if b { 1 } else { 0 });
        }
    }

    #[test]
    fn prop_num_set_matches_acts_len(n in 1..2000usize, seed in any::<u64>()) {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut ba = BitArray::new(n);
        ba.random_set_num(&mut rng, n / 4);

        prop_assert_eq!(ba.num_set(), ba.get_acts().len());
    }

    #[test]
    fn prop_clear_all_zeros(n in 1..2000usize) {
        let mut ba = BitArray::new(n);
        ba.set_all();
        ba.clear_all();

        prop_assert_eq!(ba.num_set(), 0);
    }

    #[test]
    fn prop_set_all_fills(n in 1..2000usize) {
        let mut ba = BitArray::new(n);
        ba.set_all();

        prop_assert_eq!(ba.num_set(), n);
    }

    #[test]
    fn prop_toggle_twice_identity(n in 1..1000usize, bit in 0..999usize) {
        if bit >= n { return Ok(()); }

        let mut ba = BitArray::new(n);
        let initial = ba.get_bit(bit);
        ba.toggle_bit(bit);
        ba.toggle_bit(bit);

        prop_assert_eq!(ba.get_bit(bit), initial);
    }

    #[test]
    fn prop_and_commutative(n in 32..512usize, seed in any::<u64>()) {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut ba1 = BitArray::new(n);
        let mut ba2 = BitArray::new(n);

        ba1.random_set_pct(&mut rng, 0.3);
        ba2.random_set_pct(&mut rng, 0.3);

        let r1 = &ba1 & &ba2;
        let r2 = &ba2 & &ba1;

        prop_assert_eq!(r1, r2);
    }

    #[test]
    fn prop_or_commutative(n in 32..512usize, seed in any::<u64>()) {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut ba1 = BitArray::new(n);
        let mut ba2 = BitArray::new(n);

        ba1.random_set_pct(&mut rng, 0.3);
        ba2.random_set_pct(&mut rng, 0.3);

        let r1 = &ba1 | &ba2;
        let r2 = &ba2 | &ba1;

        prop_assert_eq!(r1, r2);
    }

    #[test]
    fn prop_double_negation(n in 32..512usize, seed in any::<u64>()) {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut ba = BitArray::new(n);
        ba.random_set_pct(&mut rng, 0.5);

        let result = !&!&ba;
        prop_assert_eq!(result, ba);
    }
}

// =============================================================================
// Serialization Tests (if serde feature enabled)
// =============================================================================

#[test]
fn test_clone() {
    let mut ba = BitArray::new(1024);
    ba.random_set_num(&mut rand::rngs::StdRng::seed_from_u64(0), 100);

    let ba_clone = ba.clone();
    assert_eq!(ba, ba_clone);
}
