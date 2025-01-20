use crate::systematic::KMAX;
use thiserror::Error;
use std::cmp::min;

#[derive(Debug, Error)]
pub enum BlockError {
    #[error("Invalid block parameters")]
    InvalidParameters,
    #[error("Transfer length too large")]
    TransferTooLarge,
}

/// Parameters for source block construction as defined in Section 5.3.1.2
#[derive(Debug, Clone)]
pub struct BlockParameters {
    /// Transfer length in bytes
    pub transfer_length: u64,
    /// Symbol alignment in bytes
    pub alignment: usize,
    /// Symbol size in bytes (must be multiple of alignment)
    pub symbol_size: usize,
    /// Number of source blocks
    pub num_blocks: usize,
    /// Number of sub-blocks per source block
    pub num_subblocks: usize,
}

impl BlockParameters {
    /// Create new block parameters following Section 4.2
    pub fn new(
        transfer_length: u64,
        target_subblock_size: usize,
        max_payload_size: usize,
        alignment: usize,
        max_symbols_per_packet: usize
    ) -> Result<Self, BlockError> {
        // Validate input parameters
        if alignment == 0 || max_payload_size % alignment != 0 {
            return Err(BlockError::InvalidParameters);
        }

        // Calculate parameters following Section 4.2
        let g = min(
            ((max_payload_size as f64 * 1024.0 / transfer_length as f64).ceil() as usize),
            max_payload_size / alignment,
            max_symbols_per_packet
        );

        let symbol_size = (max_payload_size / (alignment * g)) * alignment;
        let kt = ((transfer_length as f64) / (symbol_size as f64)).ceil() as usize;

        // Calculate number of source blocks
        let num_blocks = (kt as f64 / KMAX as f64).ceil() as usize;

        // Calculate number of sub-blocks
        let num_subblocks = min(
            ((kt as f64 / num_blocks as f64 * symbol_size as f64) / target_subblock_size as f64).ceil() as usize,
            symbol_size / alignment
        );

        if kt * symbol_size as u64 > transfer_length {
            return Err(BlockError::TransferTooLarge);
        }

        Ok(Self {
            transfer_length,
            alignment,
            symbol_size,
            num_blocks,
            num_subblocks,
        })
    }
}

/// Represents a source block with its sub-blocks
#[derive(Debug)]
pub struct SourceBlock {
    /// Block number
    pub number: usize,
    /// Symbols in this block
    pub symbols: Vec<Vec<u8>>,
    /// Sub-blocks for this source block
    pub sub_blocks: Vec<Vec<Vec<u8>>>,
}

impl SourceBlock {
    /// Create a new source block from input data following Section 5.3.1.2
    pub fn new(
        data: &[u8],
        block_number: usize,
        params: &BlockParameters
    ) -> Result<Self, BlockError> {
        let block_size = params.symbol_size * (data.len() / params.symbol_size);
        let mut symbols = Vec::new();

        // Split data into symbols
        for i in 0..(block_size / params.symbol_size) {
            let start = i * params.symbol_size;
            let end = start + params.symbol_size;
            symbols.push(data[start..end].to_vec());
        }

        // Add padding to last symbol if needed
        if data.len() % params.symbol_size != 0 {
            let mut last_symbol = data[block_size..].to_vec();
            last_symbol.resize(params.symbol_size, 0);
            symbols.push(last_symbol);
        }

        // Create sub-blocks
        let mut sub_blocks = vec![Vec::new(); params.num_subblocks];
        let sub_symbol_size = params.symbol_size / params.num_subblocks;

        for symbol in &symbols {
            for (i, sub_block) in sub_blocks.iter_mut().enumerate() {
                let start = i * sub_symbol_size;
                let end = start + sub_symbol_size;
                sub_block.push(symbol[start..end].to_vec());
            }
        }

        Ok(Self {
            number: block_number,
            symbols,
            sub_blocks,
        })
    }

    /// Get sub-symbol from block
    pub fn sub_symbol(&self, symbol_index: usize, sub_block: usize) -> Option<&[u8]> {
        self.sub_blocks
            .get(sub_block)?
            .get(symbol_index)
            .map(|s| s.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_parameters() {
        let params = BlockParameters::new(
            1_000_000,    // 1MB transfer
            8192,         // 8KB target sub-block
            1024,         // 1KB max payload
            4,           // 4-byte alignment
            10,          // max 10 symbols per packet
        );
        
        assert!(params.is_ok());
        let params = params.unwrap();
        assert_eq!(params.symbol_size % params.alignment, 0);
    }

    #[test]
    fn test_source_block_creation() {
        let params = BlockParameters {
            transfer_length: 1000,
            alignment: 4,
            symbol_size: 100,
            num_blocks: 1,
            num_subblocks: 2,
        };

        let data = vec![1u8; 250];
        let block = SourceBlock::new(&data, 0, &params);
        
        assert!(block.is_ok());
        let block = block.unwrap();
        assert_eq!(block.sub_blocks.len(), 2);
        assert!(block.sub_blocks[0].len() > 0);
    }

    #[test]
    fn test_sub_symbol_access() {
        let params = BlockParameters {
            transfer_length: 1000,
            alignment: 4,
            symbol_size: 100,
            num_blocks: 1,
            num_subblocks: 2,
        };

        let data = vec![1u8; 250];
        let block = SourceBlock::new(&data, 0, &params).unwrap();
        
        assert!(block.sub_symbol(0, 0).is_some());
        assert!(block.sub_symbol(0, 2).is_none());
    }
}
