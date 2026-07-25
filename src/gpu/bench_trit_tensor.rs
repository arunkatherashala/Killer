use crate::gpu::TritTensor;
use std::time::Instant;

pub fn run_bench() {
    let n = 256;
    let a = TritTensor::from_vec(n, n, vec![1i8; n * n]);
    let b = TritTensor::from_vec(n, n, vec![1i8; n * n]);
    let start = Instant::now();
    let mut s = 0i64;
    for i in 0..n {
        for j in 0..n {
            s += a.dot_row_col(i, &b, j) as i64;
        }
    }
    let dur = start.elapsed();
    println!("TritTensor bench: sum={} time_ms={}", s, dur.as_millis());
}
