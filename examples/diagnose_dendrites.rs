use gnomics::{Block, BlockOutput, blocks::ContextLearner};
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let mut learner = ContextLearner::new(10, 4, 8, 32, 20, 20, 2, 1, 2, true, 42);
    
    let input_out = Rc::new(RefCell::new(BlockOutput::new()));
    let context_out = Rc::new(RefCell::new(BlockOutput::new()));
    
    input_out.borrow_mut().setup(2, 10);
    context_out.borrow_mut().setup(2, 40);
    
    learner.input_mut().add_child(input_out.clone(), 0);
    learner.context_mut().add_child(context_out.clone(), 0);
    learner.init().unwrap();
    
    // Set pattern
    input_out.borrow_mut().state.set_bit(0);
    input_out.borrow_mut().state.set_bit(1);
    context_out.borrow_mut().state.set_bit(5);
    context_out.borrow_mut().state.set_bit(10);
    input_out.borrow_mut().store();
    context_out.borrow_mut().store();
    
    println!("Context active bits: {:?}", context_out.borrow().state.get_acts());
    println!("Num context bits: {}", context_out.borrow().state.num_set());
    println!("Dendrite threshold: 20");
    println!("Receptors per dendrite: 32");
    println!("\n=== PROBLEM ===");
    println!("Only {} active context bits, but threshold is 20!", context_out.borrow().state.num_set());
    println!("Even after learning, max overlap = {} < 20", context_out.borrow().state.num_set());
    println!("Dendrites can NEVER fire!\n");
    
    // Run iterations
    for i in 1..=3 {
        learner.step();
        learner.pull();
        learner.compute();
        learner.store();
        learner.learn();
        
        println!("Iteration {}: anomaly={:.3}, historical_count={}", 
                 i, learner.get_anomaly_score(), learner.get_historical_count());
    }
    
    println!("\n=== Solution ===");
    println!("Need to either:");
    println!("1. Increase number of active context bits (use denser encoding)");
    println!("2. Decrease dendrite threshold to match available bits");
    println!("3. Use smaller num_rpd (receptors per dendrite)");
}
