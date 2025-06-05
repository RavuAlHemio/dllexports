mod error;
pub mod fat;
mod huff;
mod io_util;
mod kwaj;
mod ring_buffer;


use std::io::{Read, Write};

use crate::error::DecompressionError;


pub fn decompress<R: Read, W: Write>(
    compressed_reader: &mut R,
    decompressed_writer: &mut W,
) -> Result<(), DecompressionError> {
    let mut magic_buf = [0u8; 8];
    compressed_reader.read_exact(&mut magic_buf)?;
    if &magic_buf == b"KWAJ\x88\xF0\x27\xD1" {
        crate::kwaj::decompress(compressed_reader, decompressed_writer)
    } else {
        Err(DecompressionError::UnknownCompressionMethod)
    }
}
