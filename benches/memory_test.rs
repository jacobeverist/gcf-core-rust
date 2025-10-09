//! Memory Usage Analysis for BitArray
//!
//! This example demonstrates BitArray memory consumption patterns
//! by creating arrays of different sizes and reporting their memory usage.
//! Useful for understanding the relationship between bit count and actual
//! memory allocation.
//use bitvec::prelude::*;

use gnomics::{bitarray_copy_words, BitArray, Word, BITS_PER_WORD};

/// Number of test iterations with increasing array sizes
const NUM_TEST_ITERATIONS: usize = 10;

/// Base size in bits for the smallest test array
const BASE_SIZE_BITS: usize = 1024;

fn main() {
    println!("BitArray Memory Usage Analysis");
    println!("==============================");
    println!("Testing arrays from {} to {} bits\n",
             BASE_SIZE_BITS,
             NUM_TEST_ITERATIONS * BASE_SIZE_BITS);

    println!("{:>10} | {:>12} | {:>15}", "Bits", "Bytes", "Bytes/Bit");
    println!("{:-<10}-+-{:-<12}-+-{:-<15}", "", "", "");

    for iteration in 1..NUM_TEST_ITERATIONS {
        let num_bits = iteration * BASE_SIZE_BITS;
        let array = BitArray::new(num_bits);
        let memory_bytes = array.memory_usage();
        let bytes_per_bit = memory_bytes as f64 / num_bits as f64;

        //let bits = bits![WordType, Msb0; 0; num_bits];
        //let arr = bitarr![WordType, Lsb0; 0; num_bits];

        println!("{:>10} | {:>12} | {:>15.4} | {:>12}",
                 num_bits,
                 memory_bytes,
                 bytes_per_bit,
                 std::mem::size_of::<BitArray>())
        }

    println!("\nNote: Overhead includes Vec metadata and BitArray struct size");
}
