use lazy_static::lazy_static;

/// Q = 65521, largest prime smaller than 2^16
pub const Q: u32 = 65521;

lazy_static! {
    /// V0 lookup table from Section 5.6.1
    pub static ref V0: [u32; 256] = [
        251291136, 3952231631, 3370958628, 4070167936, 123631495, 3351110283,
        3218676425, 2011642291, 774603218, 2402805061, 1004366930,
        1843948209, 428891132, 3746331984, 1591258008, 3067016507,
        // ... rest of V0 table from RFC 5053
    ];

    /// V1 lookup table from Section 5.6.2
    pub static ref V1: [u32; 256] = [
        807385413, 2043073223, 3336749796, 1302105833, 2278607931, 541015020,
        1684564270, 372709334, 3508252125, 1768346005, 1270451292,
        2603029534, 2049387273, 3891424859, 2152948345, 4114760273,
        // ... rest of V1 table from RFC 5053  
    ];
}

/// Random number generator defined in Section 5.4.4.1
pub fn rand(x: u32, i: u32, m: u32) -> u32 {
    let v0 = V0[(x + i) as usize % 256];
    let v1 = V1[((x / 256) + i) as usize % 256];
    (v0 ^ v1) % m
}

/// Degree generator defined in Section 5.4.4.2
pub fn deg(v: u32) -> u32 {
    // f[j-1] <= v < f[j] then Deg[v] = d[j]
    let f = [0, 10241, 491582, 712794, 831695, 948446, 1032189, 1048576];
    let d = [0, 1, 2, 3, 4, 10, 11, 40];

    for j in 1..8 {
        if v < f[j] {
            return d[j];
        }
    }
    d[7] // Maximum degree
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rand_generator() {
        assert!(rand(123456, 0, 100) < 100);
        assert_eq!(rand(1, 1, 2), rand(1, 1, 2)); // Deterministic
    }

    #[test]
    fn test_degree_generator() {
        assert_eq!(deg(0), 1);
        assert_eq!(deg(10240), 1);
        assert_eq!(deg(10241), 2);
        assert_eq!(deg(1048575), 40);
    }
}
