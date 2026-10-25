//! matrix.rs — ALGEBRA ya MATRIX kwa vifaa vidogo (pure Rust, hakuna BLAS).
//!
//! Hii ni safu ya msingi ya hesabu za matrix zinazotumiwa na CALCULUS na CORTEX.
//! Row-major, f32, na kanuni za MTECHOS: hakuna `.unwrap()`/`panic!`/`..` kwenye
//! code ya runtime — kila kitu hurejesha `Option`/`Result` au sufuri salama.

/// Matrix ya row-major (rows × cols) kwenye `f32`.
#[derive(Clone, Debug, PartialEq)]
pub struct Mat {
    pub rows: usize,
    pub cols: usize,
    /// `data[i * cols + j]` = kipengele (i, j).
    pub data: Vec<f32>,
}

impl Mat {
    /// Matrix tupu (sufuri) yenye `rows × cols`.
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Mat {
            rows,
            cols,
            data: vec![0.0; rows.saturating_mul(cols)],
        }
    }

    /// Matrix ya utambulisho (identity) `n × n`.
    pub fn identity(n: usize) -> Self {
        let mut m = Mat::zeros(n, n);
        for i in 0..n.min(m.rows) {
            m.data[i * n + i] = 1.0;
        }
        m
    }

    /// Tengeneza kutoka vector (hukagua urefu).
    pub fn from_vec(rows: usize, cols: usize, data: Vec<f32>) -> Option<Self> {
        if data.len() != rows.saturating_mul(cols) {
            return None;
        }
        Some(Mat { rows, cols, data })
    }

    #[inline]
    pub fn idx(&self, i: usize, j: usize) -> Option<usize> {
        if i < self.rows && j < self.cols {
            Some(i * self.cols + j)
        } else {
            None
        }
    }

    #[inline]
    pub fn get(&self, i: usize, j: usize) -> f32 {
        match self.idx(i, j) {
            Some(k) => self.data[k],
            None => 0.0,
        }
    }

    #[inline]
    pub fn set(&mut self, i: usize, j: usize, v: f32) {
        if let Some(k) = self.idx(i, j) {
            self.data[k] = v;
        }
    }

    /// Transpose (huruhusu mraba na isiyo ya mraba).
    pub fn transpose(&self) -> Mat {
        let mut t = Mat::zeros(self.cols, self.rows);
        for i in 0..self.rows {
            for j in 0..self.cols {
                t.data[j * self.rows + i] = self.data[i * self.cols + j];
            }
        }
        t
    }

    /// Y = A · B. Huhitaji `A.cols == B.rows`.
    pub fn matmul(&self, b: &Mat) -> Option<Mat> {
        if self.cols != b.rows {
            return None;
        }
        let mut out = Mat::zeros(self.rows, b.cols);
        for i in 0..self.rows {
            for k in 0..self.cols {
                let a_ik = self.data[i * self.cols + k];
                if a_ik == 0.0 {
                    continue;
                }
                let brow = &b.data[k * b.cols..k * b.cols + b.cols];
                let orow = &mut out.data[i * b.cols..i * b.cols + b.cols];
                for j in 0..b.cols {
                    orow[j] += a_ik * brow[j];
                }
            }
        }
        Some(out)
    }

    /// y = A · x (matrix × vector).
    pub fn matvec(&self, x: &[f32]) -> Option<Vec<f32>> {
        if x.len() != self.cols {
            return None;
        }
        let mut y = vec![0.0f32; self.rows];
        for i in 0..self.rows {
            let mut acc = 0.0f32;
            let row = &self.data[i * self.cols..i * self.cols + self.cols];
            for j in 0..self.cols {
                acc += row[j] * x[j];
            }
            y[i] = acc;
        }
        Some(y)
    }

    /// Frobenius norm ‖A‖_F.
    pub fn frobenius_norm(&self) -> f32 {
        let mut s = 0.0f32;
        for v in self.data.iter() {
            s += v * v;
        }
        s.sqrt()
    }

    /// LU decomposition yenye partial pivoting: A = P·L·U.
    /// Hurudisha `(P, L, U)` ikiwa A ni mraba; vinginevyo `None`.
    pub fn lu(&self) -> Option<(Mat, Mat, Mat, i32)> {
        if self.rows != self.cols || self.rows == 0 {
            return None;
        }
        let n = self.rows;
        let mut a = self.clone();
        let mut perm: Vec<usize> = (0..n).collect();
        let mut sign = 1i32;
        for col in 0..n {
            // Chagua pivot (partial pivoting).
            let mut piv = col;
            let mut best = a.data[col * n + col].abs();
            for r in (col + 1)..n {
                let v = a.data[r * n + col].abs();
                if v > best {
                    best = v;
                    piv = r;
                }
            }
            if best <= f32::EPSILON {
                return None; // singular
            }
            if piv != col {
                perm.swap(col, piv);
                sign = -sign;
                for c in 0..n {
                    a.data.swap(col * n + c, piv * n + c);
                }
            }
            let pivot = a.data[col * n + col];
            for r in (col + 1)..n {
                let factor = a.data[r * n + col] / pivot;
                a.data[r * n + col] = factor;
                for c in (col + 1)..n {
                    a.data[r * n + c] -= factor * a.data[col * n + c];
                }
            }
        }
        let mut l = Mat::identity(n);
        let mut u = Mat::zeros(n, n);
        for i in 0..n {
            for j in 0..n {
                if i > j {
                    l.data[i * n + j] = a.data[i * n + j];
                } else {
                    u.data[i * n + j] = a.data[i * n + j];
                }
            }
        }
        let mut p = Mat::zeros(n, n);
        for (i, &pi) in perm.iter().enumerate() {
            p.data[i * n + pi] = 1.0;
        }
        Some((p, l, u, sign))
    }

    /// Suluhisha A·x = b kwa Gaussian elimination yenye partial pivoting.
    pub fn solve(&self, b: &[f32]) -> Option<Vec<f32>> {
        let n = self.rows;
        if self.rows != self.cols || b.len() != n || n == 0 {
            return None;
        }
        // Matrix ya kupanua [A | b] (n × n+1).
        let w = n + 1;
        let mut m = vec![0.0f32; n * w];
        for i in 0..n {
            m[i * w..i * w + n].copy_from_slice(&self.data[i * n..i * n + n]);
            m[i * w + n] = b[i];
        }
        for col in 0..n {
            // Chagua pivot.
            let mut piv = col;
            let mut best = m[col * w + col].abs();
            for r in (col + 1)..n {
                let v = m[r * w + col].abs();
                if v > best {
                    best = v;
                    piv = r;
                }
            }
            if best <= f32::EPSILON {
                return None; // singular
            }
            if piv != col {
                for c in 0..w {
                    m.swap(col * w + c, piv * w + c);
                }
            }
            let pivot = m[col * w + col];
            for r in (col + 1)..n {
                let factor = m[r * w + col] / pivot;
                if factor == 0.0 {
                    continue;
                }
                for c in col..w {
                    m[r * w + c] -= factor * m[col * w + c];
                }
            }
        }
        // Back substitution.
        let mut x = vec![0.0f32; n];
        for i in (0..n).rev() {
            let mut acc = m[i * w + n];
            for j in (i + 1)..n {
                acc -= m[i * w + j] * x[j];
            }
            let d = m[i * w + i];
            if d.abs() <= f32::EPSILON {
                return None;
            }
            x[i] = acc / d;
        }
        Some(x)
    }

    /// Determinant kwa kutumia LU (bidhaa ya diagonal ya U × sign).
    pub fn determinant(&self) -> Option<f32> {
        let (_, _, u, sign) = self.lu()?;
        let n = self.rows;
        let mut det = sign as f32;
        for i in 0..n {
            det *= u.data[i * n + i];
        }
        Some(det)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn matmul_ni_sahihi() {
        let a = Mat::from_vec(2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
        let b = Mat::from_vec(3, 2, vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0]).unwrap();
        let c = a.matmul(&b).unwrap();
        assert_eq!(c.rows, 2);
        assert_eq!(c.cols, 2);
        assert!(approx(c.get(0, 0), 58.0, 1e-4));
        assert!(approx(c.get(1, 1), 154.0, 1e-4));
    }

    #[test]
    fn matmul_hukata_miimo_isiyolingana() {
        let a = Mat::zeros(2, 3);
        let b = Mat::zeros(2, 2);
        assert!(a.matmul(&b).is_none());
    }

    #[test]
    fn identity_ni_upande_mbili() {
        let i = Mat::identity(3);
        let a = Mat::from_vec(3, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]).unwrap();
        let c = i.matmul(&a).unwrap();
        assert_eq!(a, c);
    }

    #[test]
    fn transpose_hubadilisha_miimo() {
        let a = Mat::from_vec(2, 3, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
        let t = a.transpose();
        assert_eq!(t.rows, 3);
        assert_eq!(t.cols, 2);
        assert!(approx(t.get(2, 1), 6.0, 1e-6));
    }

    #[test]
    fn lu_solve_inasuluhisha_mfumo() {
        // 2x + 3y = 8 ; 5x + 4y = 13  ->  x = 1, y = 2
        let a = Mat::from_vec(2, 2, vec![2.0, 3.0, 5.0, 4.0]).unwrap();
        let x = a.solve(&[8.0, 13.0]).unwrap();
        assert!(approx(x[0], 1.0, 1e-4), "x0={}", x[0]);
        assert!(approx(x[1], 2.0, 1e-4), "x1={}", x[1]);
    }

    #[test]
    fn determinant_ni_sahihi() {
        let a = Mat::from_vec(2, 2, vec![2.0, 3.0, 5.0, 4.0]).unwrap();
        assert!(approx(a.determinant().unwrap(), -7.0, 1e-4));
    }

    #[test]
    fn solve_hukata_matrix_singular() {
        let a = Mat::from_vec(2, 2, vec![1.0, 2.0, 2.0, 4.0]).unwrap();
        assert!(a.solve(&[1.0, 2.0]).is_none());
    }

    #[test]
    fn frobenius_norm_ni_sahihi() {
        let a = Mat::from_vec(1, 2, vec![3.0, 4.0]).unwrap();
        assert!(approx(a.frobenius_norm(), 5.0, 1e-5));
    }
}
