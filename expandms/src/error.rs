use std::fmt;


#[derive(Debug)]
pub enum DecompressionError {
    Io(std::io::Error),
    UnknownCompressionMethod,
    Huffman(crate::huff::HuffmanConstructionError),
    UnknownHuffmanTreeEncoding { encoding: u8 },
    UnexpectedHuffmanSymbolCount { symbol_count: usize },
    RelativeValueUnderflow,
    DataOffsetWithinHeader,
    Inflate(crate::inflate::Error),
    Lzx(lzxd::DecompressError),
}
impl fmt::Display for DecompressionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e)
                => write!(f, "I/O error: {}", e),
            Self::UnknownCompressionMethod
                => write!(f, "unknown compression method"),
            Self::Huffman(e)
                => write!(f, "Huffman tree construction error: {}", e),
            Self::UnknownHuffmanTreeEncoding { encoding }
                => write!(f, "unknown Huffman tree encoding: {:#04X}", encoding),
            Self::UnexpectedHuffmanSymbolCount { symbol_count }
                => write!(f, "unexpected symbol count {} for Huffman tree", symbol_count),
            Self::RelativeValueUnderflow
                => write!(f, "a relative value would underflow 0"),
            Self::DataOffsetWithinHeader
                => write!(f, "data offset points to a location within the header"),
            Self::Inflate(e)
                => write!(f, "Inflate error: {}", e),
            Self::Lzx(e)
                => write!(f, "LZX decompression error: {}", e),
        }
    }
}
impl std::error::Error for DecompressionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::UnknownCompressionMethod => None,
            Self::Huffman(e) => Some(e),
            Self::UnknownHuffmanTreeEncoding { .. } => None,
            Self::UnexpectedHuffmanSymbolCount { .. } => None,
            Self::RelativeValueUnderflow => None,
            Self::DataOffsetWithinHeader => None,
            Self::Inflate(e) => Some(e),
            Self::Lzx(e) => Some(e),
        }
    }
}
impl From<std::io::Error> for DecompressionError {
    fn from(value: std::io::Error) -> Self { Self::Io(value) }
}
impl From<crate::huff::HuffmanConstructionError> for DecompressionError {
    fn from(value: crate::huff::HuffmanConstructionError) -> Self { Self::Huffman(value) }
}
impl From<crate::inflate::Error> for DecompressionError {
    fn from(value: crate::inflate::Error) -> Self { Self::Inflate(value) }
}
impl From<lzxd::DecompressError> for DecompressionError {
    fn from(value: lzxd::DecompressError) -> Self { Self::Lzx(value) }
}
