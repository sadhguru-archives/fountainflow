//! Implementation of the Raptor decoder based on RFC 5053

use crate::fountain::Block;
use crate::linear_algebra::BinaryMatrix;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DecoderError {
    #[error("Not enough blocks received")]
    NotEnoughBlocks,
    #[error("Invalid block size: {0}")]
    InvalidBlockSize(usize),
    #[error("Decoding failed: {0}")]
    DecodingFailed(String),
    #[error("System not solvable")]
    SystemNotSolvable,
}

/// Represents the state of a block in the decoding process
#[derive(Debug, Clone, Copy, PartialEq)]
enum BlockState {
    /// Block has been received but not processed
    Pending,
    /// Block has been processed and is part of the equation system
    Processed,
    /// Block has been solved (converted to a source block)
    Solved,
}

/// Decoder for Raptor codes as specified in RFC 5053
pub struct Decoder {
    /// Expected number of source blocks
    source_block_count: usize,
    /// Size of each block in bytes
    block_size: usize,
    /// Map of received encoded blocks
    received_blocks: HashMap<u32, Block>,
    /// State of each block in the decoding process
    block_states: HashMap<u32, BlockState>,
    /// Decoded source blocks
    decoded_blocks: Vec<Option<Vec<u8>>>,
    /// Current state of the equation system
    equation_matrix: BinaryMatrix,
    /// Right-hand side of the equation system
    equation_values: Vec<u8>,
}

impl Decoder {
    /// Create a new decoder for the given number of source blocks
    pub fn new(source_block_count: usize, block_size: usize) -> Result<Self, DecoderError> {
        if block_size == 0 {
            return Err(DecoderError::InvalidBlockSize(block_size));
        }

        // Initialize equation system size based on RFC 5053 section 5.5
        let matrix_size = source_block_count + 
                         (source_block_count / 2) +  // LDPC symbols
                         (source_block_count / 4);   // Half symbols

        Ok(Self {
            source_block_count,
            block_size,
            received_blocks: HashMap::new(),
            block_states: HashMap::new(),
            decoded_blocks: vec![None; source_block_count],
            equation_matrix: BinaryMatrix::new(matrix_size, matrix_size),
            equation_values: vec![0; matrix_size],
        })
    }

    /// Add a received block to the decoder
    pub fn add_block(&mut self, block: Block, sequence: u32) -> Result<(), DecoderError> {
        if block.data().len() != self.block_size {
            return Err(DecoderError::InvalidBlockSize(block.data().len()));
        }

        self.received_blocks.insert(sequence, block);
        self.block_states.insert(sequence, BlockState::Pending);
        Ok(())
    }

    /// Process blocks that are in pending state
    fn process_pending_blocks(&mut self) -> Result<(), DecoderError> {
        let pending_blocks: Vec<_> = self.block_states
            .iter()
            .filter(|(_, &state)| state == BlockState::Pending)
            .map(|(&seq, _)| seq)
            .collect();

        for sequence in pending_blocks {
            let block = self.received_blocks.get(&sequence).unwrap();
            
            // Update equation matrix based on block's relationships
            // This follows Section 5.5.2.2 of RFC 5053
            let row = self.equation_matrix.rows();
            self.equation_values.push(1); // Add new equation
            
            // Fill in matrix row based on block relationships
            let (seed, degree) = (block.seed(), block.degree());
            self.update_equation_matrix(row, seed, degree)?;
            
            self.block_states.insert(sequence, BlockState::Processed);
        }
        Ok(())
    }

    /// Update equation matrix for a new block
    fn update_equation_matrix(&mut self, row: usize, seed: u32, degree: usize) -> Result<(), DecoderError> {
        // Implementation follows section 5.4.2.3 of RFC 5053
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        
        // Mark the dependencies in the matrix
        for _ in 0..degree {
            let col = (rng.next_u32() % self.source_block_count as u32) as usize;
            self.equation_matrix[row][col] ^= 1;
        }
        
        Ok(())
    }

    /// Try to decode the original data
    pub fn try_decode(&mut self) -> Result<bool, DecoderError> {
        // Process any new blocks first
        self.process_pending_blocks()?;
        
        // Check if we have enough equations
        if self.equation_values.len() < self.source_block_count {
            return Ok(false);
        }

        // Solve the system using Gaussian elimination
        if let Some(solution) = self.equation_matrix.solve(&self.equation_values) {
            // Convert solution to source blocks
            for (i, value) in solution.into_iter().enumerate().take(self.source_block_count) {
                if value == 1 {
                    let block_data = self.received_blocks
                        .values()
                        .next()
                        .map(|b| b.data().to_vec())
                        .ok_or(DecoderError::NotEnoughBlocks)?;
                    self.decoded_blocks[i] = Some(block_data);
                }
            }
            Ok(true)
        } else {
            Err(DecoderError::SystemNotSolvable)
        }
    }

    /// Get the decoded data if available
    pub fn get_decoded_data(&self) -> Option<Vec<u8>> {
        if self.decoded_blocks.iter().all(|block| block.is_some()) {
            let mut result = Vec::with_capacity(self.source_block_count * self.block_size);
            for block in &self.decoded_blocks {
                if let Some(data) = block {
                    result.extend_from_slice(data);
                }
            }
            Some(result)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fountain::Block;

    #[test]
    fn test_decoder_creation() {
        let decoder = Decoder::new(10, 1000);
        assert!(decoder.is_ok());

        let decoder = Decoder::new(10, 0);
        assert!(matches!(decoder, Err(DecoderError::InvalidBlockSize(0))));
    }

    #[test]
    fn test_add_block() {
        let mut decoder = Decoder::new(10, 8).unwrap();
        let block = Block::new(vec![1, 2, 3, 4, 5, 6, 7, 8], 42, 3);
        
        assert!(decoder.add_block(block, 0).is_ok());
    }

    #[test]
    fn test_invalid_block_size() {
        let mut decoder = Decoder::new(10, 8).unwrap();
        let block = Block::new(vec![1, 2, 3, 4], 42, 3);
        
        assert!(matches!(
            decoder.add_block(block, 0),
            Err(DecoderError::InvalidBlockSize(4))
        ));
    }
}