fn main() {
    let path = std::env::args().nth(1).unwrap();
    let src = std::fs::read_to_string(path).unwrap();
    let p = killer_native::compiler::compile_killer_default(&src).unwrap();
    for (i, ins) in p.instructions.iter().enumerate() { println!("{:3}: {:?}", i, ins); }
    println!("arities: {:?}", p.function_arities);
}
