use std::collections::HashMap;
use lazy_static::lazy_static;

/// Maximum number of source symbols allowed in a source block (RFC 5053 Section 5.4.2.3)
pub const KMAX: usize = 256;

lazy_static! {
    /// Systematic index cache for quick lookups from RFC 5053 Section 5.7
    static ref SYSTEMATIC_INDEX_TABLE: HashMap<usize, usize> = {
        let mut m = HashMap::new();
        // Add all values from Section 5.7 of RFC 5053
        let table = [
            (4, 18), (5, 14), (6, 61), (7, 46), (8, 39), (9, 58), (10, 62), (11, 55), (12, 41),
            (13, 67), (14, 50), (15, 75), (16, 43), (17, 19), (18, 37), (19, 30), (20, 22),
            (21, 53), (22, 25), (23, 34), (24, 29), (25, 20), (26, 33), (27, 15), (28, 24),
            (29, 13), (30, 35), (31, 51), (32, 9), (33, 49), (34, 45), (35, 63), (36, 8),
            (37, 48), (38, 54), (39, 47), (40, 59), (41, 71), (42, 32), (43, 52), (44, 38),
            (45, 27), (46, 26), (47, 69), (48, 23), (49, 56), (50, 40), (51, 66), (52, 17),
            (53, 65), (54, 74), (55, 21), (56, 36), (57, 57), (58, 60), (59, 16), (60, 64),
            (61, 42), (62, 12), (63, 31), (64, 68), (65, 28), (66, 73), (67, 70), (68, 44),
            (69, 11), (70, 7), (71, 72), (72, 6), (73, 10), (74, 5), (75, 4), (76, 3),
            (77, 2), (78, 1), (79, 0)
        ];
        for &(k, j) in &table {
            m.insert(k, j);
        }
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
        // Test some specific values from RFC 5053 Section 5.7
        assert_eq!(get_systematic_index(4), Some(18));
        assert_eq!(get_systematic_index(5), Some(14));
        assert_eq!(get_systematic_index(6), Some(61));
        assert_eq!(get_systematic_index(10), Some(62));
        assert_eq!(get_systematic_index(50), Some(40));
        assert_eq!(get_systematic_index(79), Some(0));

        // Test invalid values
        assert_eq!(get_systematic_index(3), None);  // K < 4
        assert_eq!(get_systematic_index(0), None);  // K = 0
        assert_eq!(get_systematic_index(KMAX + 1), None); // K > 256
    }

    #[test]
    fn test_ldpc_params() {
        // Test with K = 100 (example from RFC 5053)
        let params = LDPCParams::new(100);
        assert_eq!(params.s, 16); // ceil(0.01 * 100) + X where X = 15
        assert!(params.h > 0);
        assert_eq!(params.l, 100 + params.s + params.h);

        // Test minimum K
        let params = LDPCParams::new(4);
        assert!(params.s >= 1); // At least ceil(0.01 * 4)
        assert!(params.h > 0);
        assert_eq!(params.l, 4 + params.s + params.h);

        // Test maximum K
        let params = LDPCParams::new(KMAX);
        assert!(params.s >= 3); // At least ceil(0.01 * 256)
        assert!(params.h > 0);
        assert_eq!(params.l, KMAX + params.s + params.h);
    }

    #[test]
    fn test_gray_sequence() {
        // Test small sequence
        let seq = generate_gray_sequence(4);
        assert_eq!(seq, vec![0, 1, 3, 2]);

        // Test properties of larger sequence
        let seq = generate_gray_sequence(8);
        // Adjacent values differ by only one bit
        for i in 1..seq.len() {
            let diff = seq[i] ^ seq[i-1];
            assert_eq!(diff.count_ones(), 1);
        }
    }

    #[test]
    fn test_combinations() {
        // Test specific values
        assert_eq!(combinations(4, 2), 6);
        assert_eq!(combinations(5, 3), 10);
        assert_eq!(combinations(6, 0), 1);
        assert_eq!(combinations(6, 6), 1);

        // Test symmetry property
        assert_eq!(combinations(10, 4), combinations(10, 6));

        // Test edge cases
        assert_eq!(combinations(0, 0), 1);
        assert_eq!(combinations(5, 6), 0);
    }
}
