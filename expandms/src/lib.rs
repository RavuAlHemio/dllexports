mod error;
pub mod fat;
mod huff;
pub mod inflate;
mod io_util;
pub mod iso9660;
mod kwaj;
mod ring_buffer;
mod szdd;


use std::io::{Read, Write};

pub use crate::error::DecompressionError;


pub fn decompress<R: Read, W: Write>(
    compressed_reader: &mut R,
    decompressed_writer: &mut W,
) -> Result<(), DecompressionError> {
    let mut magic_buf = [0u8; 8];
    compressed_reader.read_exact(&mut magic_buf)?;
    if &magic_buf == b"KWAJ\x88\xF0\x27\xD1" {
        crate::kwaj::decompress(compressed_reader, decompressed_writer)
    } else if &magic_buf == b"SZDD\x88\xF0\x27\x33" {
        crate::szdd::decompress_szdd(compressed_reader, decompressed_writer)
    } else if &magic_buf == b"SZ \x88\xF0\x27\x33\xD1" {
        crate::szdd::decompress_sz(compressed_reader, decompressed_writer)
    } else {
        Err(DecompressionError::UnknownCompressionMethod)
    }
}
