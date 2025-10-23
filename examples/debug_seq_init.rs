use gnomics::{blocks::{DiscreteTransformer, SequenceLearner}, Block, Network, Result, OutputAccess, InputAccess};

fn main() -> Result<()> {
    let mut net = Network::new();

    // Create encoder with 512 output bits
    let encoder = net.add(DiscreteTransformer::new(10, 512, 2, 0));
    println!("Created DiscreteTransformer: num_v=10, num_s=512");

    // Create learner expecting 512 input bits
    let learner = net.add(SequenceLearner::new(
        512, 4, 8, 32, 20, 20, 2, 1, 2, false, 0,
    ));
    println!("Created SequenceLearner: num_c=512");

    // Connect
    net.connect_to_input(encoder, learner)?;
    println!("Connected encoder to learner");

    // Build
    net.build()?;
    println!("Built network");

    // Check encoder output size before init
    {
        let enc = net.get::<DiscreteTransformer>(encoder)?;
        let out_size = enc.output().borrow().state.num_bits();
        println!("Encoder output size: {} bits", out_size);
    }

    // Check learner input size before init
    {
        let lrn = net.get::<SequenceLearner>(learner)?;
        let in_size = lrn.input().num_bits();
        println!("Learner input size before init: {} bits", in_size);
        println!("Learner num_c: {}", 512);
    }

    // Initialize learner
    println!("Attempting to initialize learner...");
    net.get_mut::<SequenceLearner>(learner)?.init()?;
    println!("Successfully initialized learner");

    Ok(())
}
