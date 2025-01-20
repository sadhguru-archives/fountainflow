//! Implementation of degree distributions for Raptor codes
//! Based on RFC 5053 Section 5.4.4

use std::collections::HashMap;
use rand::Rng;

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
    fn build_distribution(params: &DistributionParams) -> Vec<(usize, f64)> {
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
    pub fn generate_triple(&mut self, k: usize, x: u32) -> (usize, u32, u32) {
        // Values from RFC 5053
        let q = 65521; // Largest prime < 2^16
        let j_k = self.systematic_index(k);

        // Calculate parameters based on RFC 5053
        let a = (53591 + j_k * 997) % q;
        let b = 10267 * (j_k + 1) % q;
        let y = (b + x * a) % q;
        
        let v = self.rand(y, 0, 1048576); // 2^20
        let d = self.degree_from_v(v);
        let a = 1 + self.rand(y, 1, k as u32 - 1);
        let b = self.rand(y, 2, k as u32);

        (d, a, b)
    }

    /// Random number generator specified in Section 5.4.4.1
    fn rand(&mut self, y: u32, i: u32, m: u32) -> u32 {
        // Using provided tables V0 and V1 from RFC 5053 Section 5.6
        // For this example, we'll use a simplified version
        let v0 = self.get_v0(y);
        let v1 = self.get_v1(y);
        
        ((v0 + i) ^ v1) % m
    }

    /// Get value from table V0 (simplified version)
    fn get_v0(&self, y: u32) -> u32 {
        // In practice, this would use the actual V0 table from RFC 5053
        y % 256
    }

    /// Get value from table V1 (simplified version)
    fn get_v1(&self, y: u32) -> u32 {
        // In practice, this would use the actual V1 table from RFC 5053
        (y / 256) % 256
    }

    /// Convert random value to degree based on Table 1
    fn degree_from_v(&self, v: u32) -> usize {
        let v = v as usize;
        let degree_table = [
            (0, 1),
            (10241, 2),
            (491582, 3),
            (712794, 4),
            (831695, 10),
            (948446, 11),
            (1032189, 40),
        ];
        
        for i in 0..degree_table.len() {
            if v < degree_table[i].0 {
                return degree_table[i].1;
            }
        }
        degree_table[0].1 // Default to degree 1
    }

    /// Get systematic index for a given K (Section 5.7)
    fn systematic_index(&self, k: usize) -> u32 {
        // In practice, this would use the table from Section 5.7
        // For now, we return a simplified value
        (k % 256) as u32
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

        // Verify we get all expected degrees
        assert!(counts.contains_key(&1));
        assert!(counts.contains_key(&2));
        assert!(counts.contains_key(&3));
        assert!(counts.contains_key(&4));
    }

    #[test]
    fn test_triple_generation() {
        let mut gen = DegreeGenerator::new(1000);
        
        // Test multiple triples
        for i in 0..10 {
            let (d, a, b) = gen.generate_triple(1000, i);
            
            // Basic sanity checks
            assert!(d >= 1);
            assert!(d <= 40); // Max degree from table
            assert!(a >= 1);
            assert!(a < 1000);
            assert!(b >= 0);
            assert!(b < 1000);
        }
    }
}