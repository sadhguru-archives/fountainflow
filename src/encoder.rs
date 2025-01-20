//! Implementation of the systematic Raptor encoder based on RFC 5053
//! This implements the encoding process described in Section 5.4

use crate::distribution::DegreeGenerator;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EncoderError {
    #[error("Invalid block size: {0}")]
    InvalidBlockSize(usize),
    #[error("Invalid source data length")]
    InvalidSourceLength,
}

/// Systematic Raptor encoder following RFC 5053
pub struct Encoder {
    /// Original source symbols
    source_symbols: Vec<Vec<u8>>,
    /// Number of source symbols
    k: usize,
    /// Size of each symbol in bytes
    symbol_size: usize,
    /// Degree generator for producing encoding symbol triples
    degree_generator: DegreeGenerator,
    /// Pre-calculated intermediate symbols
    intermediate_symbols: Option<Vec<Vec<u8>>>,
}

impl Encoder {
    /// Create a new encoder for the given source data
    pub fn new(data: &[u8], symbol_size: usize) -> Result<Self, EncoderError> {
        if symbol_size == 0 {
            return Err(EncoderError::InvalidBlockSize(symbol_size));
        }

        // Calculate number of source symbols
        let k = (data.len() + symbol_size - 1) / symbol_size;
        let mut source_symbols = Vec::with_capacity(k);

        // Split data into symbols
        for chunk in data.chunks(symbol_size) {
            let mut symbol = chunk.to_vec();
            // Pad last symbol if necessary
            if symbol.len() < symbol_size {
                symbol.resize(symbol_size, 0);
            }
            source_symbols.push(symbol);
        }

        Ok(Self {
            source_symbols,
            k,
            symbol_size,
            degree_generator: DegreeGenerator::new(k),
            intermediate_symbols: None,
        })
    }

    /// Generate intermediate symbols as specified in Section 5.4.2.4
    fn generate_intermediate_symbols(&mut self) -> Result<(), EncoderError> {
        // For the systematic case, we need to solve the system described in 
        // Section 5.4.2.4.2 to find the intermediate symbols

        // Calculate number of LDPC and Half symbols based on Section 5.4.2.3
        let s = (self.k as f64 * 0.01).ceil() as usize + 
                ((self.k as f64).sqrt() as usize);
        let h = (self.k as f64 / 4.0).ceil() as usize;
        
        let l = self.k + s + h;
        let mut symbols = Vec::with_capacity(l);

        // This will be expanded in future implementation to include
        // LDPC and Half symbols as per Section 5.4.2.4.2
        
        // For now, we'll use a simplified version where intermediate symbols
        // are just the source symbols padded with zeroes
        symbols.extend(self.source_symbols.clone());
        symbols.extend(vec![vec![0; self.symbol_size]; s + h]);
        
        self.intermediate_symbols = Some(symbols);
        Ok(())
    }

    /// Generate the next repair symbol
    pub fn next_repair_symbol(&mut self) -> Result<Vec<u8>, EncoderError> {
        // Ensure intermediate symbols are generated
        if self.intermediate_symbols.is_none() {
            self.generate_intermediate_symbols()?;
        }

        let intermediates = self.intermediate_symbols.as_ref().unwrap();
        let (degree, a, b) = self.degree_generator.generate_triple(self.k, 0);
        
        // Implement LT encoding as specified in Section 5.4.4.3
        let mut result = vec![0; self.symbol_size];
        let mut b = b as usize;
        
        // First symbol
        while b >= self.k {
            b = (b + a as usize) % self.k;
        }
        result.copy_from_slice(&intermediates[b]);

        // XOR remaining symbols
        for _ in 1..degree {
            b = (b + a as usize) % self.k;
            for i in 0..self.symbol_size {
                result[i] ^= intermediates[b][i];
            }
        }

        Ok(result)
    }

    /// Get a source symbol
    pub fn source_symbol(&self, index: usize) -> Option<&[u8]> {
        self.source_symbols.get(index).map(|s| s.as_slice())
    }

    /// Total number of source symbols
    pub fn source_symbols_count(&self) -> usize {
        self.k
    }

    /// Size of each symbol in bytes
    pub fn symbol_size(&self) -> usize {
        self.symbol_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_creation() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let encoder = Encoder::new(&data, 4);
        assert!(encoder.is_ok());

        let encoder = encoder.unwrap();
        assert_eq!(encoder.source_symbols_count(), 2);
        assert_eq!(encoder.symbol_size(), 4);
    }

    #[test]
    fn test_invalid_symbol_size() {
        let data = vec![1, 2, 3, 4];
        let encoder = Encoder::new(&data, 0);
        assert!(matches!(encoder, Err(EncoderError::InvalidBlockSize(0))));
    }

    #[test]
    fn test_repair_symbol_generation() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let mut encoder = Encoder::new(&data, 4).unwrap();

        let repair = encoder.next_repair_symbol();
        assert!(repair.is_ok());
        assert_eq!(repair.unwrap().len(), 4);
    }

    #[test]
    fn test_source_symbol_access() {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let encoder = Encoder::new(&data, 4).unwrap();

        assert_eq!(encoder.source_symbol(0), Some(&[1, 2, 3, 4][..]));
        assert_eq!(encoder.source_symbol(1), Some(&[5, 6, 7, 8][..]));
        assert_eq!(encoder.source_symbol(2), None);
    }
}