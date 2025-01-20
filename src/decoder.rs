//! Implementation of the Raptor decoder based on RFC 5053

use crate::fountain::Block;
use crate::linear_algebra::BinaryMatrix;
use crate::distribution::DegreeGenerator;
use crate::systematic::{LDPCParams, generate_gray_sequence};
use std::collections::HashMap;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum DecoderError {
    #[error("Not enough blocks received")]
    NotEnoughBlocks,
    #[error("Invalid block size: {0}")]
    InvalidBlockSize(usize),
    #[error("Invalid block count: {0} (must be between 4 and 256)")]
    InvalidBlockCount(usize),
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
    /// Expected number of source blocks (K)
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
    /// Degree generator for block relationships
    degree_gen: DegreeGenerator,
    /// LDPC and Half symbol parameters
    ldpc_params: LDPCParams,
    /// Gray sequence for Half symbols
    gray_sequence: Vec<usize>,
}

impl Decoder {
    /// Create a new decoder for the given number of source blocks
    ///
    /// # Arguments
    /// * `source_block_count` - Number of source blocks (K), must be in range 4..=256
    /// * `block_size` - Size of each block in bytes, must be > 0
    pub fn new(source_block_count: usize, block_size: usize) -> Result<Self, DecoderError> {
        // Validate K range (RFC 5053 requirement)
        if source_block_count < 4 || source_block_count > 256 {
            return Err(DecoderError::InvalidBlockCount(source_block_count));
        }

        if block_size == 0 {
            return Err(DecoderError::InvalidBlockSize(block_size));
        }

        // Calculate LDPC parameters
        let ldpc_params = LDPCParams::new(source_block_count);
        let matrix_size = ldpc_params.l; // Total intermediate symbols

        // Generate Gray sequence for Half symbols
        let gray_sequence = generate_gray_sequence(ldpc_params.h);

        let mut decoder = Self {
            source_block_count,
            block_size,
            received_blocks: HashMap::new(),
            block_states: HashMap::new(),
            decoded_blocks: vec![None; source_block_count],
            equation_matrix: BinaryMatrix::new(matrix_size, matrix_size),
            equation_values: vec![0; matrix_size],
            degree_gen: DegreeGenerator::new(source_block_count),
            ldpc_params,
            gray_sequence,
        };

        // Initialize constraint rows
        decoder.initialize_ldpc_constraints()?;
        decoder.initialize_half_constraints()?;

        Ok(decoder)
    }

    /// Initialize LDPC constraint rows in the equation matrix
    fn initialize_ldpc_constraints(&mut self) -> Result<(), DecoderError> {
        let k = self.source_block_count;
        let s = self.ldpc_params.s;
        
        // Add LDPC constraints following Section 5.4.2.3
        for i in 0..s {
            let row = k + i;
            
            // Each LDPC constraint connects to 3 source symbols
            let a = 1 + (i / s) * (k / s);
            let b = 1 + ((i + 1) / s) * (k / s);
            let c = 1 + ((i + 2) / s) * (k / s);

            self.equation_matrix[row][a % k] ^= 1;
            self.equation_matrix[row][b % k] ^= 1;
            self.equation_matrix[row][c % k] ^= 1;
        }

        Ok(())
    }

    /// Initialize Half symbol constraint rows in the equation matrix
    fn initialize_half_constraints(&mut self) -> Result<(), DecoderError> {
        let k = self.source_block_count;
        let s = self.ldpc_params.s;
        let h = self.ldpc_params.h;
        
        // Add Half symbol constraints following Section 5.4.2.3
        for i in 0..h {
            let row = k + s + i;
            let h_half = (h + 1) / 2;
            
            // Each Half constraint connects to ceil(h/2) source symbols
            for j in 0..h_half {
                let b = self.gray_sequence[j];
                let symbol = (b + i) % k;
                self.equation_matrix[row][symbol] ^= 1;
            }
        }

        Ok(())
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

    /// Update equation matrix for a new block following RFC 5053 Section 5.4.4.4
    fn update_equation_matrix(&mut self, row: usize, sequence: u32, degree: usize) -> Result<(), DecoderError> {
        // Generate triple (d, a, b) for this sequence number
        let triple = self.degree_gen.generate_triple(self.source_block_count, sequence)
            .ok_or_else(|| DecoderError::DecodingFailed("Invalid block count".to_string()))?;
        
        let (_, a, b) = triple;
        let k = self.source_block_count;
        
        // First block
        let mut index = (b as usize) % k;
        self.equation_matrix[row][index] ^= 1;

        // Subsequent blocks following the sequence defined in RFC 5053
        for _ in 1..degree {
            index = ((index + (a as usize)) % k) as usize;
            self.equation_matrix[row][index] ^= 1;
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
        // Test valid parameters
        let decoder = Decoder::new(100, 1000);
        assert!(decoder.is_ok());
        let decoder = decoder.unwrap();
        assert_eq!(decoder.source_block_count, 100);
        assert_eq!(decoder.block_size, 1000);

        // Test matrix size includes LDPC and Half symbols
        let expected_size = 100 + (100/2) + (100/4); // K + K/2 + K/4
        assert_eq!(decoder.equation_matrix.rows(), expected_size);
        assert_eq!(decoder.equation_matrix.cols(), expected_size);

        // Test invalid block size
        let decoder = Decoder::new(100, 0);
        assert!(matches!(decoder, Err(DecoderError::InvalidBlockSize(0))));

        // Test invalid block counts
        let decoder = Decoder::new(3, 1000);
        assert!(matches!(decoder, Err(DecoderError::InvalidBlockCount(3))));
        let decoder = Decoder::new(257, 1000);
        assert!(matches!(decoder, Err(DecoderError::InvalidBlockCount(257))));
    }

    #[test]
    fn test_constraint_initialization() {
        let decoder = Decoder::new(100, 1000).unwrap();
        let params = decoder.ldpc_params;
        
        // Verify LDPC constraints
        let mut ldpc_rows_nonzero = 0;
        for i in 100..(100 + params.s) {
            let mut row_ones = 0;
            for j in 0..100 {
                if decoder.equation_matrix[i][j] == 1 {
                    row_ones += 1;
                }
            }
            if row_ones > 0 {
                ldpc_rows_nonzero += 1;
            }
            assert_eq!(row_ones, 3); // Each LDPC row has exactly 3 ones
        }
        assert_eq!(ldpc_rows_nonzero, params.s);

        // Verify Half symbol constraints
        let mut half_rows_nonzero = 0;
        for i in (100 + params.s)..(100 + params.s + params.h) {
            let mut row_ones = 0;
            for j in 0..100 {
                if decoder.equation_matrix[i][j] == 1 {
                    row_ones += 1;
                }
            }
            if row_ones > 0 {
                half_rows_nonzero += 1;
            }
            assert_eq!(row_ones, (params.h + 1) / 2); // Each Half row has ceil(h/2) ones
        }
        assert_eq!(half_rows_nonzero, params.h);
    }

    #[test]
    fn test_block_processing() {
        let mut decoder = Decoder::new(100, 8).unwrap();
        
        // Add and process a block
        let block = Block::new(vec![1, 2, 3, 4, 5, 6, 7, 8], 0, 3);
        assert!(decoder.add_block(block, 0).is_ok());
        assert!(decoder.process_pending_blocks().is_ok());

        // Verify block state transition
        assert_eq!(decoder.block_states.get(&0), Some(&BlockState::Processed));
    }

    #[test]
    fn test_deterministic_matrix_construction() {
        let mut decoder1 = Decoder::new(100, 8).unwrap();
        let mut decoder2 = Decoder::new(100, 8).unwrap();

        // Add same block to both decoders
        let block = Block::new(vec![1, 2, 3, 4, 5, 6, 7, 8], 42, 3);
        decoder1.add_block(block.clone(), 0).unwrap();
        decoder2.add_block(block, 0).unwrap();

        decoder1.process_pending_blocks().unwrap();
        decoder2.process_pending_blocks().unwrap();

        // Verify matrices are identical (same block relationships)
        for i in 0..decoder1.equation_matrix.rows() {
            for j in 0..decoder1.equation_matrix.cols() {
                assert_eq!(decoder1.equation_matrix[i][j], decoder2.equation_matrix[i][j]);
            }
        }
    }

    #[test]
    fn test_invalid_block_size() {
        let mut decoder = Decoder::new(100, 8).unwrap();
        
        // Test block with wrong size
        let block = Block::new(vec![1, 2, 3, 4], 42, 3);
        assert!(matches!(
            decoder.add_block(block, 0),
            Err(DecoderError::InvalidBlockSize(4))
        ));
    }
}