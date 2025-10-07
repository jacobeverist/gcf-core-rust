use gnomics::blocks::{DiscreteTransformer, ContextLearner};
use gnomics::Block;

fn main() {
    // Same configuration as the failing test
    let mut input_encoder = DiscreteTransformer::new(8, 64, 2, 0);
    let mut context_encoder = DiscreteTransformer::new(8, 64, 2, 0);
    let mut learner = ContextLearner::new(64, 2, 8, 32, 20, 20, 2, 1, 2, false, 42);
    
    learner.input.add_child(input_encoder.output(), 0);
    learner.context.add_child(context_encoder.output(), 0);
    learner.init().unwrap();
    
    input_encoder.set_value(0);
    context_encoder.set_value(0);
    
    let context_active = context_encoder.output().borrow().state.num_set();
    println!("Context active bits: {}", context_active);
    println!("Dendrite threshold: 20");
    println!("Receptors per dendrite: 32");
    println!();
    println!("Problem: After learning, only {} receptors can connect", context_active);
    println!("to active context bits (one per active bit).");
    println!("Maximum overlap = {} < threshold (20)", context_active);
    println!("Therefore, dendrites can NEVER fire!");
    println!();
    
    // Run and verify
    input_encoder.execute(false).unwrap();
    context_encoder.execute(false).unwrap();
    learner.execute(true).unwrap();
    println!("After 1st execution: anomaly = {:.3}", learner.get_anomaly_score());
    
    input_encoder.execute(false).unwrap();
    context_encoder.execute(false).unwrap();
    learner.execute(true).unwrap();
    println!("After 2nd execution: anomaly = {:.3}", learner.get_anomaly_score());
    
    input_encoder.execute(false).unwrap();
    context_encoder.execute(false).unwrap();
    learner.execute(true).unwrap();
    println!("After 3rd execution: anomaly = {:.3}", learner.get_anomaly_score());
}
