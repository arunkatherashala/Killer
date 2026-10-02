use killer_native::builtin::BuiltinFunctions;
use killer_native::value::Value;

fn range(args: &[f64]) -> Vec<f64> {
    let vals: Vec<Value> = args.iter().map(|n| Value::Number(*n)).collect();
    match BuiltinFunctions::call_by_id(36, &vals).unwrap() {
        Value::Array(a) => a.to_vec().iter().map(|v| if let Value::Number(n) = v { *n } else { f64::NAN }).collect(),
        other => panic!("range returned {:?}", other),
    }
}

#[test]
fn single_argument_is_zero_to_n() {
    assert_eq!(range(&[5.0]), vec![0.0, 1.0, 2.0, 3.0, 4.0]);
    assert!(range(&[0.0]).is_empty());
    assert!(range(&[-3.0]).is_empty());
}

#[test]
fn start_end_and_step_forms() {
    assert_eq!(range(&[2.0, 5.0]), vec![2.0, 3.0, 4.0]);
    assert_eq!(range(&[1.0, 10.0, 3.0]), vec![1.0, 4.0, 7.0]);
    assert_eq!(range(&[5.0, 0.0, -2.0]), vec![5.0, 3.0, 1.0]);
}
