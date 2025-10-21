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
    for i in 0..25 {
        context_out.borrow_mut().state.set_bit(i);
    }
    input_out.borrow_mut().store();
    context_out.borrow_mut().store();
    
    println!("Input columns active: {:?}", input_out.borrow().state.get_acts());
    println!("Context bits active: {}", context_out.borrow().state.num_set());
    println!("Dendrite threshold: 20");
    println!("Receptors per dendrite: 32");
    println!();
    
    // First execution - should have surprise
    println!("=== Execution 1 (learning=true) ===");
    learner.step();
    learner.pull();
    learner.compute();
    println!("Anomaly: {:.3}", learner.get_anomaly_score());
    println!("Output statelets: {:?}", learner.output().borrow().state.get_acts());
    learner.store();
    learner.learn();
    println!("Historical count: {}", learner.get_historical_count());
    
    // Second execution - should recognize
    println!("\n=== Execution 2 (learning=true) ===");
    learner.step();
    input_out.borrow_mut().step();
    context_out.borrow_mut().step();
    input_out.borrow_mut().store();
    context_out.borrow_mut().store();
    
    learner.pull();
    println!("After pull - input: {:?}, context bits: {}", 
             learner.input().state.get_acts(), learner.context().state.num_set());
    
    learner.compute();
    println!("Anomaly: {:.3}", learner.get_anomaly_score());
    println!("Output statelets: {:?}", learner.output().borrow().state.get_acts());
    learner.store();
    learner.learn();
    println!("Historical count: {}", learner.get_historical_count());
    
    // Continue for more iterations
    for i in 3..=10 {
        println!("\n=== Execution {} ===", i);
        learner.step();
        input_out.borrow_mut().step();
        context_out.borrow_mut().step();
        input_out.borrow_mut().store();
        context_out.borrow_mut().store();
        
        learner.pull();
        learner.compute();
        learner.store();
        learner.learn();
        
        println!("Anomaly: {:.3}, Historical count: {}", 
                 learner.get_anomaly_score(), learner.get_historical_count());
    }
}
