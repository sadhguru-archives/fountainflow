use std::collections::HashMap;
use lazy_static::lazy_static;

/// Maximum number of source symbols allowed in a source block
pub const KMAX: usize = 8192;

lazy_static! {
    /// Systematic index cache for quick lookups
    static ref SYSTEMATIC_INDEX_TABLE: HashMap<usize, usize> = {
        let mut m = HashMap::new();
        // Add initial values from Section 5.7 of RFC 5053
        m.insert(4, 18);
        m.insert(5, 14);
        m.insert(6, 61);
        // More values will be added from the RFC table
        m
    };
}

/// Get systematic index J(K) for a given K value
pub fn get_systematic_index(k: usize) -> Option<usize> {
    if k < 4 || k > KMAX {
        None
    } else {
        SYSTEMATIC_INDEX_TABLE.get(&k).copied()
    }
}

/// Constants for LDPC computation as defined in Section 5.4.2.3
pub struct LDPCParams {
    pub s: usize,  // Number of LDPC symbols
    pub h: usize,  // Number of Half symbols
    pub l: usize,  // Total number of intermediate symbols
}

impl LDPCParams {
    /// Calculate LDPC parameters for given K as per Section 5.4.2.3
    pub fn new(k: usize) -> Self {
        let x = {
            let mut x = 1;
            while x * (x - 1) < 2 * k {
                x += 1;
            }
            x
        };

        // S = ceil(0.01*K) + X
        let s = (k as f64 * 0.01).ceil() as usize + x;

        // H is the smallest positive integer such that choose(H,ceil(H/2)) >= K + S
        let h = {
            let mut h = 1;
            while combinations(h, (h + 1) / 2) < k + s {
                h += 1;
            }
            h
        };

        let l = k + s + h;

        Self { s, h, l }
    }
}

/// Calculate binomial coefficient (n choose k)
fn combinations(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    if k == 0 || k == n {
        return 1;
    }
    let k = k.min(n - k);
    let mut c = 1;
    for i in 0..k {
        c = c * (n - i) / (i + 1);
    }
    c
}

/// Generate Gray sequence needed for Half symbol generation
pub fn generate_gray_sequence(length: usize) -> Vec<usize> {
    let mut sequence = Vec::with_capacity(length);
    for i in 0..length {
        sequence.push(i ^ (i >> 1));
    }
    sequence
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systematic_index_lookup() {
        assert_eq!(get_systematic_index(4), Some(18));
        assert_eq!(get_systematic_index(5), Some(14));
        assert_eq!(get_systematic_index(6), Some(61));
        assert_eq!(get_systematic_index(3), None);
        assert_eq!(get_systematic_index(KMAX + 1), None);
    }

    #[test]
    fn test_ldpc_params() {
        let params = LDPCParams::new(1024);
        assert!(params.s >= 10); // At least 1% of K
        assert!(params.h > 0);
        assert_eq!(params.l, 1024 + params.s + params.h);
    }

    #[test]
    fn test_gray_sequence() {
        let seq = generate_gray_sequence(4);
        assert_eq!(seq, vec![0, 1, 3, 2]);
    }

    #[test]
    fn test_combinations() {
        assert_eq!(combinations(4, 2), 6);
        assert_eq!(combinations(5, 3), 10);
        assert_eq!(combinations(6, 0), 1);
        assert_eq!(combinations(6, 6), 1);
    }
}
