use gnomics::{Block, BlockOutput, ContextAccess, InputAccess, OutputAccess, blocks::ContextLearner};
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    println!("=== Test 1: Using execute() ===");
    {
        let mut learner = ContextLearner::new(10, 4, 8, 32, 20, 20, 2, 1, 2, true, 42);
        
        let input_out = Rc::new(RefCell::new(BlockOutput::new()));
        let context_out = Rc::new(RefCell::new(BlockOutput::new()));
        
        input_out.borrow_mut().setup(2, 10);
        context_out.borrow_mut().setup(2, 40);
        
        learner.input_mut().add_child(input_out.clone(), 0);
        learner.context_mut().add_child(context_out.clone(), 0);
        learner.init().unwrap();
        
        // Set pattern ONCE
        input_out.borrow_mut().state.set_bit(0);
        input_out.borrow_mut().state.set_bit(1);
        for i in 0..25 {
            context_out.borrow_mut().state.set_bit(i);
        }
        input_out.borrow_mut().store();
        context_out.borrow_mut().store();
        
        // First execution
        learner.execute(true).unwrap();
        let first_anomaly = learner.get_anomaly_score();
        let first_count = learner.get_historical_count();
        println!("After 1st execute: anomaly={:.3}, count={}", first_anomaly, first_count);
        
        // Repeat 10 times using execute()
        for i in 2..=11 {
            learner.execute(true).unwrap();
            println!("After {}th execute: anomaly={:.3}, count={}", 
                     i, learner.get_anomaly_score(), learner.get_historical_count());
        }
        
        let last_anomaly = learner.get_anomaly_score();
        let last_count = learner.get_historical_count();
        
        println!("\nResult: first_anomaly={:.3}, last_anomaly={:.3}", first_anomaly, last_anomaly);
        println!("Result: first_count={}, last_count={}", first_count, last_count);
        println!("Anomaly decreased: {}", last_anomaly < first_anomaly);
        println!("Count grew: {}", last_count > first_count);
    }
    
    println!("\n=== Test 2: Using manual step/pull/compute/store/learn ===");
    {
        let mut learner = ContextLearner::new(10, 4, 8, 32, 20, 20, 2, 1, 2, true, 42);
        
        let input_out = Rc::new(RefCell::new(BlockOutput::new()));
        let context_out = Rc::new(RefCell::new(BlockOutput::new()));
        
        input_out.borrow_mut().setup(2, 10);
        context_out.borrow_mut().setup(2, 40);
        
        learner.input_mut().add_child(input_out.clone(), 0);
        learner.context_mut().add_child(context_out.clone(), 0);
        learner.init().unwrap();
        
        // Set pattern ONCE
        input_out.borrow_mut().state.set_bit(0);
        input_out.borrow_mut().state.set_bit(1);
        for i in 0..25 {
            context_out.borrow_mut().state.set_bit(i);
        }
        input_out.borrow_mut().store();
        context_out.borrow_mut().store();
        
        // First execution
        learner.execute(true).unwrap();
        let first_anomaly = learner.get_anomaly_score();
        let first_count = learner.get_historical_count();
        println!("After 1st execute: anomaly={:.3}, count={}", first_anomaly, first_count);
        
        // Repeat 10 times using manual calls
        for i in 2..=11 {
            learner.step();
            learner.pull();
            learner.compute();
            learner.store();
            learner.learn();
            println!("After {}th manual: anomaly={:.3}, count={}", 
                     i, learner.get_anomaly_score(), learner.get_historical_count());
        }
        
        let last_anomaly = learner.get_anomaly_score();
        let last_count = learner.get_historical_count();
        
        println!("\nResult: first_anomaly={:.3}, last_anomaly={:.3}", first_anomaly, last_anomaly);
        println!("Result: first_count={}, last_count={}", first_count, last_count);
        println!("Anomaly decreased: {}", last_anomaly < first_anomaly);
        println!("Count grew: {}", last_count > first_count);
    }
}
