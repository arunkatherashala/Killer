#![allow(dead_code)]
use std::simd::{Simd, SimdInt};

pub struct TritTensor {
    // store trits as i8 values -1,0,1
    pub data: Vec<i8>,
    pub rows: usize,
    pub cols: usize,
}

impl TritTensor {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self { data: vec![0i8; rows * cols], rows, cols }
    }

    pub fn from_vec(rows: usize, cols: usize, data: Vec<i8>) -> Self {
        assert_eq!(rows * cols, data.len());
        Self { data, rows, cols }
    }

    // Simple CPU SIMD-accelerated dot product for a row x column
    pub fn dot_row_col(&self, row: usize, other: &TritTensor, col: usize) -> i32 {
        assert_eq!(self.cols, other.rows);
        let mut acc: i32 = 0;
        let n = self.cols;
        let mut i = 0;
        while i + 16 <= n {
            let a_chunk = Simd::<i8, 16>::from_slice(&self.data[row * self.cols + i..row * self.cols + i + 16]);
            let mut b_chunk = [0i8; 16];
            for k in 0..16 { b_chunk[k] = other.data[k * other.cols + col]; }
            let b_simd = Simd::<i8, 16>::from_array(b_chunk);
            let prod = a_chunk.cast::<i16>() * b_simd.cast::<i16>();
            // sum lanes
            for lane in 0..16 {
                acc += prod[lane] as i32;
            }
            i += 16;
        }
        // tail
        while i < n {
            let a = self.data[row * self.cols + i] as i32;
            let b = other.data[i * other.cols + col] as i32;
            acc += a * b;
            i += 1;
        }
        acc
    }
}
