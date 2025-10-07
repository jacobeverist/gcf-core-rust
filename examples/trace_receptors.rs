use gnomics::{BlockMemory, BitArray};
use rand::SeedableRng;
use rand::rngs::StdRng;

fn main() {
    // Same config as ContextLearner
    let num_d = 10 * 4 * 8;  // num_c * num_spc * num_dps = 320 dendrites
    let num_rpd = 32;
    let perm_thr = 20;
    let perm_inc = 2;
    let perm_dec = 1;
    let pct_learn = 1.0;
    
    let mut memory = BlockMemory::new(num_d, num_rpd, perm_thr, perm_inc, perm_dec, pct_learn);
    let mut rng = StdRng::seed_from_u64(42);
    memory.init(40, &mut rng);  // 40 bits total context
    
    // Create context pattern with 25 active bits
    let mut context = BitArray::new(40);
    for i in 0..25 {
        context.set_bit(i);
    }
    
    println!("Context active bits: {}", context.num_set());
    println!("Dendrite threshold: {}", perm_thr);
    println!("Receptors per dendrite: {}", num_rpd);
    
    // Test dendrite 0
    let d = 0;
    
    println!("\n=== Before learning ===");
    let overlap_before = memory.overlap(d, &context);
    println!("Overlap: {}", overlap_before);
    
    println!("\n=== After 1 learning iteration ===");
    memory.learn_move(d, &context, &mut rng);
    let overlap_1 = memory.overlap(d, &context);
    println!("Overlap: {}", overlap_1);
    println!("Fires? {}", overlap_1 >= perm_thr as usize);
    
    println!("\n=== After 2 learning iterations ===");
    memory.learn_move(d, &context, &mut rng);
    let overlap_2 = memory.overlap(d, &context);
    println!("Overlap: {}", overlap_2);
    println!("Fires? {}", overlap_2 >= perm_thr as usize);
    
    println!("\n=== After 3 learning iterations ===");
    memory.learn_move(d, &context, &mut rng);
    let overlap_3 = memory.overlap(d, &context);
    println!("Overlap: {}", overlap_3);
    println!("Fires? {}", overlap_3 >= perm_thr as usize);
}
