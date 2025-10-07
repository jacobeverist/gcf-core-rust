use gnomics::blocks::DiscreteTransformer;
use gnomics::Block;

fn main() {
    let mut encoder = DiscreteTransformer::new(8, 64, 2, 0);
    
    encoder.set_value(0);
    
    println!("=== Testing encoder change detection ===");
    for i in 1..=5 {
        encoder.execute(false).unwrap();
        let changed = encoder.output().borrow().has_changed();
        println!("Execution {}: has_changed = {}", i, changed);
    }
    
    println!("\n=== Now change the value ===");
    encoder.set_value(1);
    encoder.execute(false).unwrap();
    let changed = encoder.output().borrow().has_changed();
    println!("After set_value(1): has_changed = {}", changed);
}
