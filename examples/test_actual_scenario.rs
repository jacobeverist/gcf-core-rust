use gnomics::blocks::{DiscreteTransformer, ContextLearner};
use gnomics::Block;

fn main() {
    let mut input_encoder = DiscreteTransformer::new(8, 64, 2, 0);
    let mut context_encoder = DiscreteTransformer::new(8, 64, 2, 0);
    let mut learner = ContextLearner::new(64, 2, 8, 32, 20, 20, 2, 1, 2, false, 42);
    
    learner.input.add_child(input_encoder.get_output(), 0);
    learner.context.add_child(context_encoder.get_output(), 0);
    learner.init().unwrap();
    
    // SAME AS FAILING TEST: set_value ONCE before loop
    input_encoder.set_value(0);
    context_encoder.set_value(0);
    
    println!("=== Simulating the failing test ===");
    for i in 1..=5 {
        input_encoder.execute(false).unwrap();
        context_encoder.execute(false).unwrap();
        
        let input_changed = input_encoder.get_output().borrow().has_changed();
        let context_changed = context_encoder.get_output().borrow().has_changed();
        
        learner.execute(true).unwrap();
        let anomaly = learner.get_anomaly_score();
        
        println!("Iteration {}: input_changed={}, context_changed={}, anomaly={:.3}", 
                 i, input_changed, context_changed, anomaly);
    }
    
    println!("\n=== Diagnosis ===");
    println!("After first execution, inputs don't change (has_changed=false)");
    println!("ContextLearner.compute() checks children_changed() and SKIPS processing");
    println!("Anomaly score remains at 1.000 from the first execution");
    println!("\nSolution: Set always_update=true to force processing even when inputs unchanged");
}
