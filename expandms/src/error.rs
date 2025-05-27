use std::fmt;


#[derive(Debug)]
pub enum DecompressionError {
    Io(std::io::Error),
    Huffman(crate::huff::HuffmanConstructionError),
    UnknownHuffmanTreeEncoding { encoding: u8 },
    UnexpectedHuffmanSymbolCount { symbol_count: usize },
    RelativeValueUnderflow,
}
impl fmt::Display for DecompressionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e)
                => write!(f, "I/O error: {}", e),
            Self::Huffman(e)
                => write!(f, "Huffman tree construction error: {}", e),
            Self::UnknownHuffmanTreeEncoding { encoding }
                => write!(f, "unknown Huffman tree encoding: {:#04X}", encoding),
            Self::UnexpectedHuffmanSymbolCount { symbol_count }
                => write!(f, "unexpected symbol count {} for Huffman tree", symbol_count),
            Self::RelativeValueUnderflow
                => write!(f, "a relative value would underflow 0"),
        }
    }
}
impl std::error::Error for DecompressionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Huffman(e) => Some(e),
            Self::UnknownHuffmanTreeEncoding { .. } => None,
            Self::UnexpectedHuffmanSymbolCount { .. } => None,
            Self::RelativeValueUnderflow => None,
        }
    }
}
impl From<std::io::Error> for DecompressionError {
    fn from(value: std::io::Error) -> Self { Self::Io(value) }
}
impl From<crate::huff::HuffmanConstructionError> for DecompressionError {
    fn from(value: crate::huff::HuffmanConstructionError) -> Self { Self::Huffman(value) }
}
