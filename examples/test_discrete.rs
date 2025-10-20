use gnomics::blocks::DiscreteTransformer;
use gnomics::Block;

fn main() {
    let mut encoder = DiscreteTransformer::new(8, 64, 2, 0);
    encoder.set_value(0);
    encoder.execute(false).unwrap();
    
    let output = encoder.get_output();
    let num_active = output.borrow().state.num_set();
    let active_bits = output.borrow().state.get_acts();
    
    println!("DiscreteTransformer(8 categories, 64 bits)");
    println!("Value: 0");
    println!("Active bits: {}", num_active);
    println!("Active indices: {:?}", active_bits);
}
