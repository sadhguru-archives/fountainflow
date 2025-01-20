//! Implementation of the fountain code algorithm based on RFC 5053 (Raptor codes)

use rand::Rng;
use thiserror::Error;
use crate::distribution::DegreeGenerator;

#[derive(Error, Debug)]
pub enum FountainError {
    #[error("Invalid block size: {0}")]
    InvalidBlockSize(usize),
    #[error("Invalid degree: {0}")]
    InvalidDegree(usize),
    #[error("Encoding error: {0}")]
    EncodingError(String),
}

/// A block of encoded data
#[derive(Debug, Clone)]
pub struct Block {
    /// The encoded data
    data: Vec<u8>,
    /// Random seed used for block generation
    seed: u32,
    /// Number of source blocks combined
    degree: usize,
}

impl Block {
    pub fn new(data: Vec<u8>, seed: u32, degree: usize) -> Self {
        Self { data, seed, degree }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn seed(&self) -> u32 {
        self.seed
    }

    pub fn degree(&self) -> usize {
        self.degree
    }
}

/// Fountain code encoder implementing Raptor codes
pub struct Encoder {
    /// Source data split into blocks
    blocks: Vec<Vec<u8>>,
    /// Size of each block
    block_size: usize,
    /// Degree generator for Raptor code distribution
    degree_gen: DegreeGenerator,
    /// Current block sequence number
    sequence: u32,
}

impl Encoder {
    /// Create a new encoder
    ///
    /// # Arguments
    /// * `data` - Source data to encode
    /// * `block_size` - Size of each block
    ///
    /// # Errors
    /// Returns error if:
    /// - block_size is 0 or larger than data length
    /// - number of blocks is outside valid range (4..=256)
    pub fn new(data: &[u8], block_size: usize) -> Result<Self, FountainError> {
        if block_size == 0 {
            return Err(FountainError::InvalidBlockSize(block_size));
        }
        if block_size > data.len() {
            return Err(FountainError::InvalidBlockSize(block_size));
        }

        let blocks: Vec<Vec<u8>> = data
            .chunks(block_size)
            .map(|chunk| chunk.to_vec())
            .collect();

        // RFC 5053 requires K (number of source blocks) to be in range 4..=256
        let k = blocks.len();
        if k < 4 || k > 256 {
            return Err(FountainError::InvalidBlockSize(block_size));
        }

        Ok(Self {
            blocks,
            block_size,
            degree_gen: DegreeGenerator::new(k),
            sequence: 0,
        })
    }

    /// Generate the next encoded block following RFC 5053 Section 5.4.4.4
    pub fn next_block(&mut self) -> Result<Block, FountainError> {
        // Generate triple (d, a, b) for current sequence number
        let triple = self.degree_gen.generate_triple(self.blocks.len(), self.sequence)
            .ok_or_else(|| FountainError::EncodingError("Invalid block count".to_string()))?;
        
        let (degree, a, b) = triple;
        
        // Select source blocks based on triple
        let selected_blocks = self.select_blocks(degree, a, b);
        
        // XOR the selected blocks together
        let mut data = vec![0u8; self.block_size];
        for block in selected_blocks {
            for (i, &byte) in block.iter().enumerate() {
                data[i] ^= byte;
            }
        }

        // Create block and increment sequence
        let block = Block::new(data, self.sequence, degree);
        self.sequence += 1;
        
        Ok(block)
    }

    /// Select source blocks based on triple values from RFC 5053 Section 5.4.4.4
    fn select_blocks(&self, degree: usize, a: u32, b: u32) -> Vec<&Vec<u8>> {
        let mut result = Vec::with_capacity(degree);
        let k = self.blocks.len();
        
        // First block
        let mut index = (b as usize) % k;
        result.push(&self.blocks[index]);

        // Subsequent blocks
        for _ in 1..degree {
            index = ((index + (a as usize)) % k) as usize;
            result.push(&self.blocks[index]);
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_creation() {
        // Test valid block count (4 blocks)
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let encoder = Encoder::new(&data, 2).unwrap();
        assert_eq!(encoder.blocks.len(), 4);
        assert_eq!(encoder.block_size, 2);
        assert_eq!(encoder.sequence, 0);
    }

    #[test]
    fn test_invalid_parameters() {
        // Test invalid block size
        let data = vec![1, 2, 3, 4];
        assert!(matches!(
            Encoder::new(&data, 0),
            Err(FountainError::InvalidBlockSize(0))
        ));
        assert!(matches!(
            Encoder::new(&data, 5),
            Err(FountainError::InvalidBlockSize(5))
        ));

        // Test invalid block count (K < 4)
        let data = vec![1, 2, 3];
        assert!(matches!(
            Encoder::new(&data, 1),
            Err(FountainError::InvalidBlockSize(1))
        ));

        // Test invalid block count (K > 256)
        let data = vec![0; 1024];
        assert!(matches!(
            Encoder::new(&data, 2),
            Err(FountainError::InvalidBlockSize(2))
        ));
    }

    #[test]
    fn test_block_generation() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let mut encoder = Encoder::new(&data, 2).unwrap();
        
        // Test first block
        let block = encoder.next_block().unwrap();
        assert_eq!(block.data().len(), 2);
        assert!(block.degree() >= 1 && block.degree() <= 40); // Valid degree range
        assert_eq!(block.seed(), 0); // First sequence number

        // Test sequence progression
        let block2 = encoder.next_block().unwrap();
        assert_eq!(block2.seed(), 1); // Second sequence number
    }

    #[test]
    fn test_deterministic_generation() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let mut encoder1 = Encoder::new(&data, 2).unwrap();
        let mut encoder2 = Encoder::new(&data, 2).unwrap();

        // Same sequence number should produce identical blocks
        let block1 = encoder1.next_block().unwrap();
        let block2 = encoder2.next_block().unwrap();

        assert_eq!(block1.data(), block2.data());
        assert_eq!(block1.degree(), block2.degree());
        assert_eq!(block1.seed(), block2.seed());
    }
}
