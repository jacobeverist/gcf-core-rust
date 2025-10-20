use bitvec::prelude::*;
use gnomics::BitField;
fn main() {

    let ba = BitField::new(1024);

    println!("{:?} {:?} {:?}",
             ba.num_bits(),
             ba.memory_usage(),
             std::mem::size_of::<BitField>()
    );

    // let bv = bitvec![0, 1, 0, 0, 1, 0, 0, 0];
    // let mut bv: BitVec = BitVec::with_capacity(128);
    let bv: BitVec = BitVec::with_capacity(1024);

    println!("{:?} {:?} {:?}",
             bv.len(),
             bv.capacity(),
             std::mem::size_of::<BitVec>()
    );

    let bv = BitVec::<u32, Msb0>::repeat(false, 1024);
    //let mut bv: BitVec = BitVec::<u32,Lsb0>::repeat(false, 1024);

    println!("{:?} {:?} {:?}",
             bv.len(),
             bv.capacity(),
             std::mem::size_of::<BitVec>()
    );


}

