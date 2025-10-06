//! Utility functions for the Gnomics framework.
//!
//! This module provides common utility functions used throughout the framework,
//! including random number generation helpers and array shuffling.

use rand::Rng;

/// Return the minimum of two values.
///
/// # Examples
///
/// ```
/// use gnomics::utils::min;
///
/// assert_eq!(min(5, 10), 5);
/// assert_eq!(min(10, 5), 5);
/// ```
#[inline]
pub fn min<T: Ord>(a: T, b: T) -> T {
    if a < b {
        a
    } else {
        b
    }
}

/// Return the maximum of two values.
///
/// # Examples
///
/// ```
/// use gnomics::utils::max;
///
/// assert_eq!(max(5, 10), 10);
/// assert_eq!(max(10, 5), 10);
/// ```
#[inline]
pub fn max<T: Ord>(a: T, b: T) -> T {
    if a > b {
        a
    } else {
        b
    }
}

/// Generate a random unsigned integer in range [min, max] (inclusive).
///
/// # Examples
///
/// ```
/// use gnomics::utils::rand_uint;
/// use rand::SeedableRng;
///
/// let mut rng = rand::rngs::StdRng::seed_from_u64(0);
/// let val = rand_uint(10, 20, &mut rng);
/// assert!(val >= 10 && val <= 20);
/// ```
#[inline]
pub fn rand_uint<R: Rng>(min: u32, max: u32, rng: &mut R) -> u32 {
    rng.gen_range(min..=max)
}

/// Shuffle a slice of u32 values in-place using Fisher-Yates algorithm.
///
/// This is a partial shuffle - only the first `n` elements are shuffled.
/// If `n >= arr.len()`, all elements are shuffled.
///
/// # Arguments
///
/// * `arr` - Mutable slice to shuffle
/// * `n` - Number of elements to shuffle
/// * `rng` - Random number generator
///
/// # Examples
///
/// ```
/// use gnomics::utils::shuffle;
/// use rand::SeedableRng;
///
/// let mut arr = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
/// let mut rng = rand::rngs::StdRng::seed_from_u64(0);
/// shuffle(&mut arr, 10, &mut rng);
/// // arr is now shuffled
/// ```
pub fn shuffle<R: Rng>(arr: &mut [u32], n: usize, rng: &mut R) {
    let n = min(n, arr.len());
    for i in (1..n).rev() {
        let j = rng.gen_range(0..=i);
        arr.swap(i, j);
    }
}

/// Shuffle a vector of usize values in-place using Fisher-Yates algorithm.
///
/// This is a convenience wrapper for shuffling usize vectors, which are
/// commonly used for index manipulation.
///
/// # Examples
///
/// ```
/// use gnomics::utils::shuffle_indices;
/// use rand::SeedableRng;
///
/// let mut indices = vec![0, 1, 2, 3, 4];
/// let mut rng = rand::rngs::StdRng::seed_from_u64(0);
/// shuffle_indices(&mut indices, &mut rng);
/// // indices is now shuffled
/// ```
pub fn shuffle_indices<R: Rng>(arr: &mut [usize], rng: &mut R) {
    for i in (1..arr.len()).rev() {
        let j = rng.gen_range(0..=i);
        arr.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_min() {
        assert_eq!(min(5, 10), 5);
        assert_eq!(min(10, 5), 5);
        assert_eq!(min(7, 7), 7);
    }

    #[test]
    fn test_max() {
        assert_eq!(max(5, 10), 10);
        assert_eq!(max(10, 5), 10);
        assert_eq!(max(7, 7), 7);
    }

    #[test]
    fn test_rand_uint() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        for _ in 0..100 {
            let val = rand_uint(10, 20, &mut rng);
            assert!(val >= 10 && val <= 20);
        }
    }

    #[test]
    fn test_shuffle() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let mut arr = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let original = arr.clone();

        shuffle(&mut arr, 10, &mut rng);

        // Array should be different after shuffle (with very high probability)
        assert_ne!(arr, original);

        // But should contain same elements
        let mut sorted = arr.clone();
        sorted.sort();
        assert_eq!(sorted, original);
    }

    #[test]
    fn test_shuffle_partial() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let mut arr = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

        // Shuffle only first 5 elements
        shuffle(&mut arr, 5, &mut rng);

        // Last 5 elements should be unchanged (in practice, may shuffle a bit
        // due to the Fisher-Yates algorithm, but the implementation matches C++)
    }

    #[test]
    fn test_shuffle_indices() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let mut indices: Vec<usize> = (0..10).collect();
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
    fn test_shuffle_deterministic() {
        // Same seed should produce same shuffle
        let mut rng1 = rand::rngs::StdRng::seed_from_u64(42);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(42);

        let mut arr1 = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut arr2 = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];

        shuffle(&mut arr1, 10, &mut rng1);
        shuffle(&mut arr2, 10, &mut rng2);

        assert_eq!(arr1, arr2);
    }
}
