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
    
    // First execution
    learner.execute(true).unwrap();
    println!("After 1st: anomaly={:.3}, count={}", 
             learner.get_anomaly_score(), learner.get_historical_count());
    
    // Now try stepping the outputs as well
    for i in 2..=5 {
        // Step the outputs forward in time
        input_out.borrow_mut().step();
        context_out.borrow_mut().step();
        
        // Re-store to mark as changed
        input_out.borrow_mut().store();
        context_out.borrow_mut().store();
        
        learner.step();
        learner.pull();
        learner.compute();
        learner.store();
        learner.learn();
        
        println!("After {}: anomaly={:.3}, count={}", 
                 i, learner.get_anomaly_score(), learner.get_historical_count());
    }
}
