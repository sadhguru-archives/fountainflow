//! Implementation of degree distributions for Raptor codes
//! Based on RFC 5053 Section 5.4.4

use rand::Rng;
use std::collections::HashMap;
use crate::tables::{self, Q};

/// Parameters for the robust soliton distribution
pub struct DistributionParams {
    /// Number of source symbols
    k: usize,
    /// Controls spike position, typically c * ln(k/delta) for some small c
    m: usize,
    /// Failure probability upper bound
    delta: f64,
}

impl DistributionParams {
    pub fn new(k: usize, delta: f64) -> Self {
        // Follow recommendations from RFC 5053 Section 5.4.4.2
        let c = 0.1; // Constant factor for spike position
        let m = (c * (k as f64).ln() / delta).ceil() as usize;
        Self { k, m, delta }
    }
}

/// Degree generator that implements the degree distribution from RFC 5053
pub struct DegreeGenerator {
    /// Cached probability distribution
    distribution: Vec<(usize, f64)>, // (degree, cumulative probability)
    /// Random number generator
    rng: rand::rngs::ThreadRng,
}

impl DegreeGenerator {
    /// Create a new degree generator following RFC 5053 Section 5.4.4.2
    pub fn new(k: usize) -> Self {
        let params = DistributionParams::new(k, 0.01); // Use 1% failure probability
        let distribution = Self::build_distribution(&params);
        
        Self {
            distribution,
            rng: rand::thread_rng(),
        }
    }

    /// Build the degree distribution according to Table 1 in RFC 5053
    fn build_distribution(_params: &DistributionParams) -> Vec<(usize, f64)> {
        let mut dist = Vec::new();
        let mut cum_prob = 0.0;

        // Pre-coded distribution from Table 1
        let table = [
            (1, 0.007969),
            (2, 0.493570),
            (3, 0.166220),
            (4, 0.072646),
            (5, 0.082558),
            (10, 0.056058),
            (11, 0.037229),
            (40, 0.083750),
        ];

        for &(degree, prob) in &table {
            cum_prob += prob;
            dist.push((degree, cum_prob));
        }

        // Normalize probabilities
        let total = dist.last().unwrap().1;
        for (_, prob) in dist.iter_mut() {
            *prob /= total;
        }

        dist
    }

    /// Generate a degree based on the distribution
    pub fn next_degree(&mut self) -> usize {
        let p: f64 = self.rng.gen();
        
        // Binary search for the degree
        let pos = self.distribution.partition_point(|&(_, cum_prob)| cum_prob < p);
        
        if pos >= self.distribution.len() {
            self.distribution[0].0 // Default to degree 1 if something goes wrong
        } else {
            self.distribution[pos].0
        }
    }
/// Generate the random triple (d, a, b) as specified in Section 5.4.4.4
pub fn generate_triple(&mut self, k: usize, x: u32) -> Option<(usize, u32, u32)> {
    // Get systematic index, return None if k is invalid
    let j_k = tables::systematic_index(k)?;

    // Calculate parameters based on RFC 5053
    let a = (53591 + j_k * 997) % Q;
    let b = 10267 * (j_k + 1) % Q;
    let y = (b + x * a) % Q;
    
    let v = self.rand(y, 0, 1048576); // 2^20
    let d = self.degree_from_v(v);
    let a = 1 + self.rand(y, 1, k as u32 - 1);
    let b = self.rand(y, 2, k as u32);

    Some((d, a, b))
    }

    /// Random number generator specified in Section 5.4.4.1
    fn rand(&mut self, y: u32, i: u32, m: u32) -> u32 {
        tables::rand(y, i, m)
    }

    /// Convert random value to degree based on Table 1
    fn degree_from_v(&self, v: u32) -> usize {
        tables::deg(v) as usize
    }

    /// Get systematic index for a given K (Section 5.7)
    fn systematic_index(&self, k: usize) -> u32 {
        tables::systematic_index(k).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_degree_distribution() {
        let mut gen = DegreeGenerator::new(1000);
        let mut counts = HashMap::new();
        
        // Generate a large number of samples
        for _ in 0..10000 {
            let degree = gen.next_degree();
            *counts.entry(degree).or_insert(0) += 1;
        }

        // Verify we get some expected degrees
        let expected_degrees = [1, 2, 3, 4, 5, 10, 11, 40];
        for degree in expected_degrees.iter() {
            assert!(counts.contains_key(degree));
        }

        // Verify all generated degrees are within the expected set
        for (degree, _) in counts.iter() {
            assert!(expected_degrees.contains(degree));
        }
    }

    #[test]
    fn test_triple_generation() {
        let mut gen = DegreeGenerator::new(100);
        
        // Test valid K value
        let triple = gen.generate_triple(100, 0);
        assert!(triple.is_some());
        let (d, a, b) = triple.unwrap();
        assert!(d >= 1 && d <= 40); // Valid degree range
        assert!(a >= 1 && a < 100); // Valid a range
        assert!(b >= 0 && b < 100); // Valid b range

        // Test deterministic generation
        let triple1 = gen.generate_triple(100, 42).unwrap();
        let triple2 = gen.generate_triple(100, 42).unwrap();
        assert_eq!(triple1, triple2); // Same input should give same output

        // Test invalid K values
        assert!(gen.generate_triple(3, 0).is_none()); // K < 4
        assert!(gen.generate_triple(257, 0).is_none()); // K > 256
    }

    #[test]
    fn test_triple_rfc_values() {
        // Test vector from RFC 5053 Section 5.4.4.4
        let mut gen = DegreeGenerator::new(100);
        let triple = gen.generate_triple(100, 2);
        assert!(triple.is_some());
        let (d, a, b) = triple.unwrap();
        
        // The RFC doesn't provide complete test vectors, but we can verify
        // the values are within valid ranges and deterministic
        assert!(d >= 1 && d <= 40);
        assert!(a >= 1 && a < 100);
        assert!(b >= 0 && b < 100);

        // Verify deterministic generation with specific seed
        let triple2 = gen.generate_triple(100, 2).unwrap();
        assert_eq!((d, a, b), triple2);
    }
}