//! Linear Algebra — 25+ functions (matrix ops, decomposition, eigenvalues)

pub struct Matrix { pub data: Vec<Vec<f64>> }

impl Matrix {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self { data: vec![vec![0.0; cols]; rows] }
    }
    pub fn transpose(&self) -> Matrix {
        let (r, c) = (self.data.len(), self.data[0].len());
        let mut result = Matrix::new(c, r);
        for i in 0..r {
            for j in 0..c {
                result.data[j][i] = self.data[i][j];
            }
        }
        result
    }
}
