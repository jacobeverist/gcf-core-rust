use gnomics::{Block, BlockOutput, blocks::ContextLearner};
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let mut learner = ContextLearner::new(10, 4, 8, 32, 20, 20, 2, 1, 2, true, 42);
    
    let input_out = Rc::new(RefCell::new(BlockOutput::new()));
    let context_out = Rc::new(RefCell::new(BlockOutput::new()));
    
    input_out.borrow_mut().setup(2, 10);
    context_out.borrow_mut().setup(2, 40);
    
    learner.input.add_child(input_out.clone(), 0);
    learner.context.add_child(context_out.clone(), 0);
    learner.init().unwrap();
    
    // Set pattern
    input_out.borrow_mut().state.set_bit(0);
    input_out.borrow_mut().state.set_bit(1);
    for i in 0..25 {
        context_out.borrow_mut().state.set_bit(i);
    }
    input_out.borrow_mut().store();
    context_out.borrow_mut().store();
    
    println!("=== First execution ===");
    learner.execute(true).unwrap();
    println!("Anomaly: {:.3}", learner.get_anomaly_score());
    println!("Historical count: {}", learner.get_historical_count());
    println!("Output: {}", learner.output.borrow().state.num_set());
    
    println!("\n=== Manual loop ===");
    for i in 1..=5 {
        println!("\n--- Iteration {} ---", i);
        learner.step();  // Advance time
        println!("After step - output changed flag: {}", input_out.borrow().has_changed());
        
        learner.pull();  // Pull from children
        println!("After pull - input bits: {:?}", learner.input.state.get_acts());
        println!("After pull - children_changed: {}", learner.input.children_changed());
        
        learner.compute();
        println!("After compute - anomaly: {:.3}", learner.get_anomaly_score());
        
        learner.store();
        println!("After store - output bits: {:?}", learner.output.borrow().state.get_acts());
        
        learner.learn();
        println!("After learn - historical count: {}", learner.get_historical_count());
    }
}
