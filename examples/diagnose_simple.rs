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
    
    // Set pattern ONCE
    input_out.borrow_mut().state.set_bit(0);
    input_out.borrow_mut().state.set_bit(1);
    context_out.borrow_mut().state.set_bit(5);
    context_out.borrow_mut().state.set_bit(10);
    input_out.borrow_mut().store();
    context_out.borrow_mut().store();
    
    println!("=== Iteration 1 ===");
    learner.execute(true).unwrap();
    println!("Anomaly: {:.3}", learner.get_anomaly_score());
    println!("Input state after pull: {:?}", learner.input.state.get_acts());
    println!("Context state after pull: {:?}", learner.context.state.get_acts());
    
    println!("\n=== Manual loop (like the test) ===");
    for i in 2..=5 {
        println!("\n--- Iteration {} ---", i);
        learner.step();
        
        // Check what's in the inputs before pull
        println!("Before pull - input state: {:?}", learner.input.state.get_acts());
        println!("Before pull - context state: {:?}", learner.context.state.get_acts());
        
        learner.pull();
        
        // Check what's in the inputs after pull
        println!("After pull - input state: {:?}", learner.input.state.get_acts());
        println!("After pull - context state: {:?}", learner.context.state.get_acts());
        println!("Input children_changed: {}", learner.input.children_changed());
        println!("Context children_changed: {}", learner.context.children_changed());
        
        learner.compute();
        println!("Anomaly: {:.3}", learner.get_anomaly_score());
        
        learner.store();
        learner.learn();
    }
}
