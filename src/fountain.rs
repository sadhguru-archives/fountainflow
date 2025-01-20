//! Implementation of the fountain code algorithm based on RFC 5053 (Raptor codes)

use rand::Rng;
use thiserror::Error;

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
    /// Random number generator
    rng: rand::rngs::ThreadRng,
}

impl Encoder {
    /// Create a new encoder
    ///
    /// # Arguments
    /// * `data` - Source data to encode
    /// * `block_size` - Size of each block
    ///
    /// # Errors
    /// Returns error if block_size is 0 or larger than data length
    pub fn new(data: &[u8], block_size: usize) -> Result<Self, FountainError> {
        if block_size == 0 {
            return Err(FountainError::InvalidBlockSize(block_size));
        }
        if block_size > data.len() {
            return Err(FountainError::InvalidBlockSize(block_size));
        }

        let blocks = data
            .chunks(block_size)
            .map(|chunk| chunk.to_vec())
            .collect();

        Ok(Self {
            blocks,
            block_size,
            rng: rand::thread_rng(),
        })
    }

    /// Generate the next encoded block
    pub fn next_block(&mut self) -> Block {
        let degree = self.get_degree();
        let seed = self.rng.gen();
        
        // Select random source blocks based on degree
        let selected_blocks = self.select_blocks(seed, degree);
        
        // XOR the selected blocks together
        let mut data = vec![0u8; self.block_size];
        for block in selected_blocks {
            for (i, &byte) in block.iter().enumerate() {
                data[i] ^= byte;
            }
        }

        Block::new(data, seed, degree)
    }

    /// Get a degree from the Robust Soliton distribution
    fn get_degree(&self) -> usize {
        // Simplified version - will be replaced with proper Robust Soliton
        self.rng.gen_range(1..=self.blocks.len())
    }

    /// Select source blocks based on seed and degree
    fn select_blocks(&self, seed: u32, degree: usize) -> Vec<&Vec<u8>> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let mut indices: Vec<usize> = (0..self.blocks.len()).collect();
        indices.shuffle(&mut rng);
        indices.truncate(degree);
        
        indices.iter().map(|&i| &self.blocks[i]).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_creation() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let encoder = Encoder::new(&data, 2).unwrap();
        assert_eq!(encoder.blocks.len(), 4);
        assert_eq!(encoder.block_size, 2);
    }

    #[test]
    fn test_invalid_block_size() {
        let data = vec![1, 2, 3, 4];
        assert!(matches!(
            Encoder::new(&data, 0),
            Err(FountainError::InvalidBlockSize(0))
        ));
        assert!(matches!(
            Encoder::new(&data, 5),
            Err(FountainError::InvalidBlockSize(5))
        ));
    }

    #[test]
    fn test_block_generation() {
        let data = vec![1, 2, 3, 4];
        let mut encoder = Encoder::new(&data, 2).unwrap();
        
        let block = encoder.next_block();
        assert_eq!(block.data().len(), 2);
        assert!(block.degree() >= 1 && block.degree() <= 2);
    }
}
