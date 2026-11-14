// ================================================================
// LINEAR ALGEBRA SOLVER - Phase 21.2
// Comprehensive matrix operations, eigenvalues, decompositions
// Ported from: solver_linear_algebra.killer
// ================================================================

use std::f64;

pub type Matrix = Vec<Vec<f64>>;
pub type Vector = Vec<f64>;

/// Linear Algebra Solver
pub struct LinearAlgebraSolver;

impl LinearAlgebraSolver {
    // ================================================================
    // BASIC MATRIX OPERATIONS (1-10)
    // ================================================================

    /// Problem 1: Matrix Multiplication C = A × B
    pub fn multiply(a: &Matrix, b: &Matrix) -> Result<Matrix, String> {
        let m = a.len();
        if m == 0 { return Err("Matrix A is empty".to_string()); }
        let n = a[0].len();
        let p = b.get(0).map(|r| r.len()).unwrap_or(0);
        
        if b.len() != n {
            return Err(format!("Dimension mismatch: A[m×{}] × B[{}×p]", n, b.len()));
        }
        
        let mut c = vec![vec![0.0; p]; m];
        for i in 0..m {
            for j in 0..p {
                for k in 0..n {
                    c[i][j] += a[i][k] * b[k][j];
                }
            }
        }
        Ok(c)
    }

    /// Problem 2: Matrix Addition C = A + B
    pub fn add(a: &Matrix, b: &Matrix) -> Result<Matrix, String> {
        if a.len() != b.len() || (a.is_empty() || a[0].len() != b[0].len()) {
            return Err("Matrices must have same dimensions".to_string());
        }
        
        let mut c = a.clone();
        for i in 0..a.len() {
            for j in 0..a[0].len() {
                c[i][j] += b[i][j];
            }
        }
        Ok(c)
    }

    /// Problem 3: Matrix Subtraction C = A - B
    pub fn subtract(a: &Matrix, b: &Matrix) -> Result<Matrix, String> {
        if a.len() != b.len() || (a.is_empty() || a[0].len() != b[0].len()) {
            return Err("Matrices must have same dimensions".to_string());
        }
        
        let mut c = a.clone();
        for i in 0..a.len() {
            for j in 0..a[0].len() {
                c[i][j] -= b[i][j];
            }
        }
        Ok(c)
    }

    /// Problem 4: Scalar Multiplication B = α·A
    pub fn scale(a: &Matrix, alpha: f64) -> Matrix {
        a.iter().map(|row| row.iter().map(|&x| x * alpha).collect()).collect()
    }

    /// Problem 5: Matrix Transpose B = A^T
    pub fn transpose(a: &Matrix) -> Matrix {
        if a.is_empty() { return vec![]; }
        let m = a.len();
        let n = a[0].len();
        let mut b = vec![vec![0.0; m]; n];
        for i in 0..m {
            for j in 0..n {
                b[j][i] = a[i][j];
            }
        }
        b
    }

    /// Problem 6: Identity Matrix
    pub fn identity(n: usize) -> Matrix {
        let mut i = vec![vec![0.0; n]; n];
        for j in 0..n {
            i[j][j] = 1.0;
        }
        i
    }

    /// Problem 7: Trace tr(A) = sum of diagonal elements
    pub fn trace(a: &Matrix) -> f64 {
        a.iter().enumerate()
            .map(|(i, row)| row.get(i).cloned().unwrap_or(0.0))
            .sum()
    }

    /// Problem 8: Determinant (LU decomposition method)
    pub fn determinant(a: &Matrix) -> Result<f64, String> {
        if a.is_empty() || a[0].len() != a.len() {
            return Err("Matrix must be square".to_string());
        }
        
        let n = a.len();
        let mut lu = a.clone();
        let mut det = 1.0;
        let mut swaps = 0;
        
        // LU decomposition with partial pivoting
        for col in 0..n {
            // Find pivot
            let mut pivot_row = col;
            for row in col + 1..n {
                if lu[row][col].abs() > lu[pivot_row][col].abs() {
                    pivot_row = row;
                }
            }
            
            if lu[pivot_row][col].abs() < 1e-14 {
                return Ok(0.0);
            }
            
            if pivot_row != col {
                lu.swap(col, pivot_row);
                swaps += 1;
            }
            
            det *= lu[col][col];
            
            // Eliminate below
            for row in col + 1..n {
                let factor = lu[row][col] / lu[col][col];
                for j in col + 1..n {
                    lu[row][j] -= factor * lu[col][j];
                }
            }
        }
        
        if swaps % 2 == 1 {
            det = -det;
        }
        
        Ok(det)
    }

    /// Problem 9: Frobenius Norm ||A||_F = sqrt(sum of squares)
    pub fn frobenius_norm(a: &Matrix) -> f64 {
        a.iter().flat_map(|row| row.iter())
            .map(|x| x * x)
            .sum::<f64>()
            .sqrt()
    }

    /// Problem 10: Element-wise Maximum
    pub fn element_max(a: &Matrix, b: &Matrix) -> Result<Matrix, String> {
        if a.len() != b.len() || (a.is_empty() || a[0].len() != b[0].len()) {
            return Err("Dimensions mismatch".to_string());
        }
        
        let mut c = a.clone();
        for i in 0..a.len() {
            for j in 0..a[0].len() {
                c[i][j] = a[i][j].max(b[i][j]);
            }
        }
        Ok(c)
    }

    // ================================================================
    // MATRIX DECOMPOSITION (11-25)
    // ================================================================

    /// Problem 11: Gaussian Elimination (forward elimination)
    pub fn gaussian_elimination(a: &Matrix, b: &Vector) -> Result<Vector, String> {
        let n = a.len();
        if b.len() != n {
            return Err("Dimension mismatch".to_string());
        }
        
        let mut aug = a.clone();
        let mut b_copy = b.clone();
        
        // Forward elimination with partial pivoting
        for col in 0..n {
            // Find pivot
            let mut pivot = col;
            for row in col + 1..n {
                if aug[row][col].abs() > aug[pivot][col].abs() {
                    pivot = row;
                }
            }
            
            aug.swap(col, pivot);
            b_copy.swap(col, pivot);
            
            if aug[col][col].abs() < 1e-14 {
                return Err("Singular matrix".to_string());
            }
            
            // Eliminate
            for row in col + 1..n {
                let factor = aug[row][col] / aug[col][col];
                for j in col..n {
                    aug[row][j] -= factor * aug[col][j];
                }
                b_copy[row] -= factor * b_copy[col];
            }
        }
        
        // Back substitution
        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            x[i] = b_copy[i];
            for j in i + 1..n {
                x[i] -= aug[i][j] * x[j];
            }
            x[i] /= aug[i][i];
        }
        
        Ok(x)
    }

    /// Problem 12: LU Decomposition A = LU
    pub fn lu_decomposition(a: &Matrix) -> Result<(Matrix, Matrix), String> {
        let n = a.len();
        if n == 0 || a[0].len() != n {
            return Err("Matrix must be square".to_string());
        }
        
        let mut l = vec![vec![0.0; n]; n];
        let mut u = a.clone();
        
        for i in 0..n {
            l[i][i] = 1.0;
            
            for j in i + 1..n {
                let factor = u[j][i] / u[i][i];
                l[j][i] = factor;
                
                for k in i..n {
                    u[j][k] -= factor * u[i][k];
                }
            }
        }
        
        Ok((l, u))
    }

    /// Problem 13: QR Decomposition (Gram-Schmidt)
    pub fn qr_decomposition(a: &Matrix) -> Result<(Matrix, Matrix), String> {
        let m = a.len();
        let n = a.get(0).map(|r| r.len()).unwrap_or(0);
        
        if n > m {
            return Err("Need m >= n for QR decomposition".to_string());
        }
        
        let mut q = vec![vec![0.0; n]; m];
        let mut r = vec![vec![0.0; n]; n];
        
        // Gram-Schmidt orthogonalization
        for j in 0..n {
            // q_j = a_j
            for i in 0..m {
                q[i][j] = a[i][j];
            }
            
            // Orthogonalize against previous vectors
            for i in 0..j {
                let mut dot = 0.0;
                for k in 0..m {
                    dot += q[k][i] * a[k][j];
                }
                r[i][j] = dot;
                
                for k in 0..m {
                    q[k][j] -= dot * q[k][i];
                }
            }
            
            // Normalize
            let mut norm = 0.0;
            for i in 0..m {
                norm += q[i][j] * q[i][j];
            }
            norm = norm.sqrt();
            
            if norm < 1e-14 {
                return Err("Linearly dependent columns".to_string());
            }
            
            r[j][j] = norm;
            for i in 0..m {
                q[i][j] /= norm;
            }
        }
        
        Ok((q, r))
    }

    /// Problem 14: Cholesky Decomposition (for SPD matrices)
    pub fn cholesky_decomposition(a: &Matrix) -> Result<Matrix, String> {
        let n = a.len();
        if n == 0 || a[0].len() != n {
            return Err("Matrix must be square".to_string());
        }
        
        let mut l = vec![vec![0.0; n]; n];
        
        for i in 0..n {
            for j in 0..=i {
                let mut sum = 0.0;
                for k in 0..j {
                    sum += l[i][k] * l[j][k];
                }
                
                if i == j {
                    let val = a[i][i] - sum;
                    if val <= 0.0 {
                        return Err("Matrix is not positive definite".to_string());
                    }
                    l[i][j] = val.sqrt();
                } else {
                    l[i][j] = (a[i][j] - sum) / l[j][j];
                }
            }
        }
        
        Ok(l)
    }

    /// Problem 15: Singular Value Decomposition (SVD) - Power iteration
    pub fn svd_power_iteration(a: &Matrix, k: usize) -> Result<(Matrix, Vector, Matrix), String> {
        let m = a.len();
        let n = a.get(0).map(|r| r.len()).unwrap_or(0);
        
        let at = Self::transpose(a);
        let mut ata = Self::multiply(&at, a)?;
        
        let mut u = vec![vec![0.0; k]; m];
        let mut sigma = vec![0.0; k];
        let mut v = vec![vec![0.0; k]; n];
        
        for i in 0..k {
            // Power iteration for largest eigenvector
            let mut x = vec![1.0; n];
            for _ in 0..20 {
                x = Self::multiply(&ata, &vec![x.clone()])?[0].clone();
                let norm = x.iter().map(|&xi| xi * xi).sum::<f64>().sqrt();
                if norm > 1e-14 {
                    x = x.iter().map(|&xi| xi / norm).collect();
                }
            }
            
            // Store singular value
            let ax = Self::multiply(&a, &vec![x.clone()])?[0].clone();
            sigma[i] = ax.iter().map(|&xi| xi * xi).sum::<f64>().sqrt();
            
            // Deflate
            for j in 0..n {
                ata[j][j] -= sigma[i] * sigma[i];
            }
        }
        
        Ok((u, sigma, v))
    }

    // ================================================================
    // LINEAR SYSTEMS (16-25)
    // ================================================================

    /// Problem 16: Solve Ax = b using LU decomposition
    pub fn solve_linear_system(a: &Matrix, b: &Vector) -> Result<Vector, String> {
        Self::gaussian_elimination(a, b)
    }

    /// Problem 17: Matrix Inverse A^(-1)
    pub fn inverse(a: &Matrix) -> Result<Matrix, String> {
        let n = a.len();
        if n == 0 || a[0].len() != n {
            return Err("Matrix must be square".to_string());
        }
        
        let i = Self::identity(n);
        let mut result = vec![vec![0.0; n]; n];
        
        for col in 0..n {
            let b: Vector = i[col].clone();
            let x = Self::solve_linear_system(a, &b)?;
            for row in 0..n {
                result[row][col] = x[row];
            }
        }
        
        Ok(result)
    }

    /// Problem 18: Least Squares Solution min ||Ax - b||
    pub fn least_squares(a: &Matrix, b: &Vector) -> Result<Vector, String> {
        let at = Self::transpose(a);
        let ata = Self::multiply(&at, a)?;
        let atb = vec![at.iter().zip(b.iter())
            .map(|(row, &bi)| row.iter().zip(std::iter::repeat(bi))
                .map(|(&aij, bj)| aij * bj).sum::<f64>())
            .collect()];
        
        Self::solve_linear_system(&ata, &atb[0])
    }

    /// Problem 19: Rank estimation
    pub fn rank(a: &Matrix, tol: f64) -> usize {
        if a.is_empty() { return 0; }
        let mut rank = 0;
        let mut row_a = a.clone();
        
        for col in 0..a[0].len() {
            let mut pivot = col;
            for row in col + 1..a.len() {
                if row_a[row][col].abs() > row_a[pivot][col].abs() {
                    pivot = row;
                }
            }
            
            if row_a[pivot][col].abs() < tol {
                continue;
            }
            
            row_a.swap(col, pivot);
            rank += 1;
            
            for row in col + 1..a.len() {
                let factor = row_a[row][col] / row_a[col][col];
                for j in col + 1..a[0].len() {
                    row_a[row][j] -= factor * row_a[col][j];
                }
            }
        }
        rank
    }

    /// Problem 20: Eigenvalue (Power iteration method)
    pub fn largest_eigenvalue(a: &Matrix, max_iter: usize) -> Result<f64, String> {
        if a.is_empty() { return Err("Empty matrix".to_string()); }
        
        let n = a.len();
        let mut x = vec![1.0; n];
        
        for _ in 0..max_iter {
            let y = Self::multiply(a, &vec![x.clone()])?[0].clone();
            let norm = y.iter().map(|&yi| yi * yi).sum::<f64>().sqrt();
            
            if norm < 1e-14 { break; }
            x = y.iter().map(|&yi| yi / norm).collect();
        }
        
        let lambda = Self::multiply(a, &vec![x.clone()])?[0]
            .iter().zip(x.iter()).map(|(&y, &x)| y / x).next()
            .unwrap_or(0.0);
        
        Ok(lambda)
    }
}

// ================================================================
// TESTS
// ================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_multiply() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let b = vec![vec![5.0, 6.0], vec![7.0, 8.0]];
        let c = LinearAlgebraSolver::multiply(&a, &b).unwrap();
        assert_eq!(c[0][0], 19.0);
        assert_eq!(c[0][1], 22.0);
        assert_eq!(c[1][0], 43.0);
        assert_eq!(c[1][1], 50.0);
    }

    #[test]
    fn test_transpose() {
        let a = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        let at = LinearAlgebraSolver::transpose(&a);
        assert_eq!(at.len(), 3);
        assert_eq!(at[0], vec![1.0, 4.0]);
    }

    #[test]
    fn test_determinant() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let det = LinearAlgebraSolver::determinant(&a).unwrap();
        assert!((det - (-2.0)).abs() < 1e-10);
    }

    #[test]
    fn test_gaussian_elimination() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let b = vec![5.0, 11.0];
        let x = LinearAlgebraSolver::gaussian_elimination(&a, &b).unwrap();
        assert!((x[0] - 1.0).abs() < 1e-10);
        assert!((x[1] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_frobenius_norm() {
        let a = vec![vec![3.0, 4.0]];
        let norm = LinearAlgebraSolver::frobenius_norm(&a);
        assert!((norm - 5.0).abs() < 1e-10);
    }
}
