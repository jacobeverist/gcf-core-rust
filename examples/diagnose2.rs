use gnomics::blocks::{DiscreteTransformer, ContextLearner};
use gnomics::Block;

fn main() {
    let mut input_encoder = DiscreteTransformer::new(8, 64, 2, 0);
    let mut context_encoder = DiscreteTransformer::new(8, 64, 2, 0);
    let mut learner = ContextLearner::new(64, 2, 8, 32, 20, 20, 2, 1, 2, false, 42);
    
    learner.input.add_child(input_encoder.get_output(), 0);
    learner.context.add_child(context_encoder.get_output(), 0);
    learner.init().unwrap();
    
    // Execute encoders first
    input_encoder.set_value(0);
    context_encoder.set_value(0);
    input_encoder.execute(false).unwrap();
    context_encoder.execute(false).unwrap();
    
    let input_active = input_encoder.get_output().borrow().state.num_set();
    let context_active = context_encoder.get_output().borrow().state.num_set();
    
    println!("=== Configuration ===");
    println!("Input active bits: {}", input_active);
    println!("Context active bits: {}", context_active);
    println!("Dendrite threshold: 20");
    println!("Receptors per dendrite: 32");
    println!();
    println!("=== Problem Diagnosis ===");
    println!("When a dendrite learns with learn_move():");
    println!("- It has 32 receptors, all starting at permanence 0");
    println!("- Only {} active bits in context", context_active);
    println!("- Each receptor at perm=0 gets moved to an active bit");
    println!("- But only {} receptors can be moved (one per active bit)", context_active);
    println!("- Remaining {} receptors stay at perm=0", 32 - context_active);
    println!();
    println!("Maximum possible overlap = {} active bits", context_active);
    println!("Dendrite threshold = 20");
    println!("Result: {} < 20, so dendrite NEVER fires!", context_active);
    println!();
    
    // Run iterations
    println!("=== Execution Results ===");
    for i in 1..=5 {
        input_encoder.execute(false).unwrap();
        context_encoder.execute(false).unwrap();
        learner.execute(true).unwrap();
        println!("Iteration {}: anomaly = {:.3}", i, learner.get_anomaly_score());
    }
}
