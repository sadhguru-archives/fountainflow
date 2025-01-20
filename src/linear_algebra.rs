//! Linear algebra operations over GF(2) as specified in RFC 5053 section 5.5
//! These operations are crucial for the decoding process

use std::ops::{Index, IndexMut};

/// Matrix over GF(2) (binary field) where operations are performed with XOR
#[derive(Debug, Clone)]
pub struct BinaryMatrix {
    rows: usize,
    cols: usize,
    data: Vec<Vec<u8>>,
}

impl BinaryMatrix {
    /// Create a new matrix with given dimensions
    pub fn new(rows: usize, cols: usize) -> Self {
        let data = vec![vec![0u8; cols]; rows];
        Self { rows, cols, data }
    }

    /// Create an identity matrix of given size
    pub fn identity(size: usize) -> Self {
        let mut matrix = Self::new(size, size);
        for i in 0..size {
            matrix[i][i] = 1;
        }
        matrix
    }

    /// Perform Gaussian elimination as described in RFC 5053 section 5.5.2
    pub fn gaussian_elimination(&mut self) -> bool {
        let mut pivot_row = 0;
        let mut pivot_col = 0;

        while pivot_row < self.rows && pivot_col < self.cols {
            // Find pivot in current column
            let mut found = false;
            for i in pivot_row..self.rows {
                if self[i][pivot_col] == 1 {
                    if i != pivot_row {
                        // Swap rows
                        for j in 0..self.cols {
                            let temp = self[i][j];
                            self[i][j] = self[pivot_row][j];
                            self[pivot_row][j] = temp;
                        }
                    }
                    found = true;
                    break;
                }
            }

            if !found {
                // No pivot found in this column, move to next
                pivot_col += 1;
                continue;
            }

            // Eliminate column entries
            for i in 0..self.rows {
                if i != pivot_row && self[i][pivot_col] == 1 {
                    // Add pivot row to current row (XOR operation)
                    for j in pivot_col..self.cols {
                        self[i][j] ^= self[pivot_row][j];
                    }
                }
            }

            pivot_row += 1;
            pivot_col += 1;
        }

        // Check if matrix has full rank
        pivot_row == self.rows
    }

    /// Solve the system Ax = b where A is this matrix
    pub fn solve(&mut self, b: &[u8]) -> Option<Vec<u8>> {
        if b.len() != self.rows {
            return None;
        }

        // Augment matrix with b
        let mut augmented = self.clone();
        for i in 0..self.rows {
            augmented.data[i].push(b[i]);
        }

        // Perform Gaussian elimination
        if !augmented.gaussian_elimination() {
            return None;
        }

        // Back substitution
        let mut x = vec![0u8; self.cols];
        for i in (0..self.rows).rev() {
            let mut sum = augmented[i][self.cols];
            for j in (i + 1)..self.cols {
                sum ^= augmented[i][j] & x[j];
            }
            x[i] = sum;
        }

        Some(x)
    }
}

impl Index<usize> for BinaryMatrix {
    type Output = Vec<u8>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl IndexMut<usize> for BinaryMatrix {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_matrix() {
        let matrix = BinaryMatrix::identity(3);
        assert_eq!(matrix[0][0], 1);
        assert_eq!(matrix[1][1], 1);
        assert_eq!(matrix[2][2], 1);
        assert_eq!(matrix[0][1], 0);
        assert_eq!(matrix[1][2], 0);
    }

    #[test]
    fn test_gaussian_elimination() {
        // Test case from RFC 5053 example
        let mut matrix = BinaryMatrix::new(3, 3);
        matrix[0] = vec![1, 1, 0];
        matrix[1] = vec![1, 0, 1];
        matrix[2] = vec![0, 1, 1];

        assert!(matrix.gaussian_elimination());
        
        // Should be in row echelon form
        assert_eq!(matrix[0][0], 1);
        assert_eq!(matrix[1][1], 1);
        assert_eq!(matrix[2][2], 1);
    }

    #[test]
    fn test_solve_system() {
        let mut matrix = BinaryMatrix::new(3, 3);
        matrix[0] = vec![1, 1, 0];
        matrix[1] = vec![1, 0, 1];
        matrix[2] = vec![0, 1, 1];

        let b = vec![1, 0, 1];
        let x = matrix.solve(&b);
        
        assert!(x.is_some());
        let x = x.unwrap();
        assert_eq!(x.len(), 3);
    }
}