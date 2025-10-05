//! Tests for utility functions.

use gnomics::utils::*;
use proptest::prelude::*;
use rand::SeedableRng;

#[test]
fn test_min() {
    assert_eq!(min(5, 10), 5);
    assert_eq!(min(10, 5), 5);
    assert_eq!(min(7, 7), 7);
    assert_eq!(min(0, 100), 0);
    assert_eq!(min(100, 0), 0);
}

#[test]
fn test_max() {
    assert_eq!(max(5, 10), 10);
    assert_eq!(max(10, 5), 10);
    assert_eq!(max(7, 7), 7);
    assert_eq!(max(0, 100), 100);
    assert_eq!(max(100, 0), 100);
}

#[test]
fn test_rand_uint_range() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);

    // Test that values are within range
    for _ in 0..100 {
        let val = rand_uint(10, 20, &mut rng);
        assert!(val >= 10 && val <= 20);
    }
}

#[test]
fn test_rand_uint_single_value() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let val = rand_uint(42, 42, &mut rng);
    assert_eq!(val, 42);
}

#[test]
fn test_rand_uint_distribution() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(12345);
    let mut histogram = vec![0; 11];

    // Generate many samples
    for _ in 0..10000 {
        let val = rand_uint(0, 10, &mut rng);
        histogram[val as usize] += 1;
    }

    // Each value should appear approximately 10000/11 ≈ 909 times
    // Check that distribution is reasonably uniform (allow ±30%)
    for count in histogram.iter() {
        assert!(*count >= 600 && *count <= 1200, "count: {}", count);
    }
}

#[test]
fn test_shuffle() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let mut arr: Vec<u32> = (0..100).collect();
    let original = arr.clone();

    shuffle(&mut arr, 100, &mut rng);

    // Should be different (with very high probability)
    assert_ne!(arr, original);

    // But should contain same elements
    let mut sorted = arr.clone();
    sorted.sort();
    assert_eq!(sorted, original);
}

#[test]
fn test_shuffle_partial() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let mut arr: Vec<u32> = (0..100).collect();

    // Shuffle only first 10 elements
    shuffle(&mut arr, 10, &mut rng);

    // All elements should still be present
    let mut sorted = arr.clone();
    sorted.sort();
    let expected: Vec<u32> = (0..100).collect();
    assert_eq!(sorted, expected);
}

#[test]
fn test_shuffle_empty() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let mut arr: Vec<u32> = vec![];
    shuffle(&mut arr, 0, &mut rng);
    assert_eq!(arr.len(), 0);
}

#[test]
fn test_shuffle_single() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let mut arr = vec![42];
    shuffle(&mut arr, 1, &mut rng);
    assert_eq!(arr, vec![42]);
}

#[test]
fn test_shuffle_deterministic() {
    // Same seed should produce same shuffle
    let mut rng1 = rand::rngs::StdRng::seed_from_u64(42);
    let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);

    let mut arr1: Vec<u32> = (0..100).collect();
    let mut arr2: Vec<u32> = (0..100).collect();

    shuffle(&mut arr1, 100, &mut rng1);
    shuffle(&mut arr2, 100, &mut rng2);

    assert_eq!(arr1, arr2);
}

#[test]
fn test_shuffle_indices() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let mut indices: Vec<usize> = (0..100).collect();
    let original = indices.clone();

    shuffle_indices(&mut indices, &mut rng);

    // Should be different
    assert_ne!(indices, original);

    // But contain same elements
    let mut sorted = indices.clone();
    sorted.sort();
    assert_eq!(sorted, original);
}

#[test]
fn test_shuffle_indices_empty() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let mut indices: Vec<usize> = vec![];
    shuffle_indices(&mut indices, &mut rng);
    assert_eq!(indices.len(), 0);
}

#[test]
fn test_shuffle_indices_deterministic() {
    let mut rng1 = rand::rngs::StdRng::seed_from_u64(99);
    let mut rng2 = rand::rngs::StdRng::seed_from_u64(99);

    let mut arr1: Vec<usize> = (0..50).collect();
    let mut arr2: Vec<usize> = (0..50).collect();

    shuffle_indices(&mut arr1, &mut rng1);
    shuffle_indices(&mut arr2, &mut rng2);

    assert_eq!(arr1, arr2);
}

// =============================================================================
// Property-Based Tests
// =============================================================================

proptest! {
    #[test]
    fn prop_min_is_minimum(a in 0..10000u32, b in 0..10000u32) {
        let result = min(a, b);
        prop_assert!(result <= a && result <= b);
        prop_assert!(result == a || result == b);
    }

    #[test]
    fn prop_max_is_maximum(a in 0..10000u32, b in 0..10000u32) {
        let result = max(a, b);
        prop_assert!(result >= a && result >= b);
        prop_assert!(result == a || result == b);
    }

    #[test]
    fn prop_rand_uint_in_range(min_val in 0..1000u32, range in 1..100u32, seed in any::<u64>()) {
        let max_val = min_val + range;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let val = rand_uint(min_val, max_val, &mut rng);
        prop_assert!(val >= min_val && val <= max_val);
    }

    #[test]
    fn prop_shuffle_preserves_elements(n in 1..100usize, seed in any::<u64>()) {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut arr: Vec<u32> = (0..n as u32).collect();
        let original = arr.clone();

        shuffle(&mut arr, n, &mut rng);

        let mut sorted = arr.clone();
        sorted.sort();
        prop_assert_eq!(sorted, original);
    }

    #[test]
    fn prop_shuffle_indices_preserves_elements(n in 1..100usize, seed in any::<u64>()) {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut arr: Vec<usize> = (0..n).collect();
        let original = arr.clone();

        shuffle_indices(&mut arr, &mut rng);

        let mut sorted = arr.clone();
        sorted.sort();
        prop_assert_eq!(sorted, original);
    }

    #[test]
    fn prop_min_max_relationship(a in 0..10000i32, b in 0..10000i32) {
        prop_assert!(min(a, b) <= max(a, b));
    }
}
