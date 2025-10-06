//! BlockBase - Common state shared by all blocks.
//!
//! This module provides the `BlockBase` structure that contains common fields
//! used by all block implementations, including unique ID, initialization flag,
//! and random number generator.

use rand::rngs::StdRng;
use rand::SeedableRng;
use std::sync::atomic::{AtomicU32, Ordering};

/// Common state shared by all blocks.
///
/// Provides unique ID generation, initialization tracking, and seeded RNG
/// for reproducible randomness.
///
/// # Examples
///
/// ```
/// use gnomics::BlockBase;
///
/// let mut base = BlockBase::new(42);
/// assert!(!base.is_initialized());
/// base.set_initialized(true);
/// assert!(base.is_initialized());
/// ```
#[derive(Clone)]
pub struct BlockBase {
    /// Unique block ID (auto-incremented)
    id: u32,
    /// Initialization flag (has init() been called?)
    init_flag: bool,
    /// Seeded random number generator (MT19937 equivalent)
    rng: StdRng,
}

impl BlockBase {
    /// Create a new BlockBase with a seed for the RNG.
    ///
    /// Each BlockBase gets a unique ID via atomic counter.
    ///
    /// # Arguments
    ///
    /// * `seed` - Seed for the random number generator
    ///
    /// # Examples
    ///
    /// ```
    /// use gnomics::BlockBase;
    ///
    /// let base1 = BlockBase::new(42);
    /// let base2 = BlockBase::new(42);
    /// // Different IDs even with same seed
    /// assert_ne!(base1.id(), base2.id());
    /// ```
    pub fn new(seed: u64) -> Self {
        static NEXT_ID: AtomicU32 = AtomicU32::new(0);

        Self {
            id: NEXT_ID.fetch_add(1, Ordering::SeqCst),
            init_flag: false,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Get the unique block ID.
    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Check if block has been initialized.
    #[inline]
    pub fn is_initialized(&self) -> bool {
        self.init_flag
    }

    /// Set initialization flag.
    ///
    /// Called by block's `init()` method to mark as initialized.
    #[inline]
    pub fn set_initialized(&mut self, flag: bool) {
        self.init_flag = flag;
    }

    /// Get mutable reference to the RNG.
    ///
    /// Allows blocks to use the RNG for random operations while
    /// maintaining reproducibility via seed.
    #[inline]
    pub fn rng(&mut self) -> &mut StdRng {
        &mut self.rng
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn test_unique_ids() {
        let base1 = BlockBase::new(0);
        let base2 = BlockBase::new(0);
        let base3 = BlockBase::new(0);

        // IDs should be unique
        assert_ne!(base1.id(), base2.id());
        assert_ne!(base2.id(), base3.id());
        assert_ne!(base1.id(), base3.id());
    }

    #[test]
    fn test_initialization_flag() {
        let mut base = BlockBase::new(0);

        assert!(!base.is_initialized());

        base.set_initialized(true);
        assert!(base.is_initialized());

        base.set_initialized(false);
        assert!(!base.is_initialized());
    }

    #[test]
    fn test_rng_deterministic() {
        let mut base1 = BlockBase::new(42);
        let mut base2 = BlockBase::new(42);

        // Same seed should produce same random sequence
        let val1a: u32 = base1.rng().gen();
        let val1b: u32 = base1.rng().gen();

        let val2a: u32 = base2.rng().gen();
        let val2b: u32 = base2.rng().gen();

        assert_eq!(val1a, val2a);
        assert_eq!(val1b, val2b);
    }

    #[test]
    fn test_rng_different_seeds() {
        let mut base1 = BlockBase::new(42);
        let mut base2 = BlockBase::new(99);

        // Different seeds should produce different sequences
        let val1: u32 = base1.rng().gen();
        let val2: u32 = base2.rng().gen();

        assert_ne!(val1, val2);
    }

    #[test]
    fn test_rng_generates_values() {
        let mut base = BlockBase::new(123);

        // Generate several values to ensure RNG works
        for _ in 0..100 {
            let _val: u32 = base.rng().gen();
        }
    }
}
