use killer_native::value::Value;
fn main() {
    println!("Value size: {} bytes", std::mem::size_of::<Value>());
}

