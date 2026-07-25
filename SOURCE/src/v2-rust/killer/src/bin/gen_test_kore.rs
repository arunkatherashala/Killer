use killer_native::kore_v2::*;

fn main() {
    let columns = vec![
        KColumn::new("id", KType::Int),
        KColumn::new("name", KType::Str),
        KColumn::new("score", KType::Float),
        KColumn::new("active", KType::Bool),
    ];
    let rows = vec![
        vec![KVal::Int(1), KVal::Str("Alice".into()), KVal::Float(95.5), KVal::Bool(true)],
        vec![KVal::Int(2), KVal::Str("Bob".into()),   KVal::Float(87.3), KVal::Bool(false)],
        vec![KVal::Int(3), KVal::Str("Alice".into()), KVal::Float(92.1), KVal::Bool(true)],
        vec![KVal::Int(4), KVal::Str("Carol".into()), KVal::Float(78.9), KVal::Bool(true)],
        vec![KVal::Int(5), KVal::Str("Bob".into()),   KVal::Float(91.0), KVal::Bool(false)],
    ];
    let path = r"C:\Users\skathera\Downloads\proof\test_v2.kore";
    let writer = KoreWriter::new(columns);
    match writer.write(path, &rows) {
        Ok(sz) => println!("Wrote {} bytes to {}", sz, path),
        Err(e) => eprintln!("Error: {}", e),
    }
}
