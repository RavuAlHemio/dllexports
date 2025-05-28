//! Decompressor for the Microsoft compression format known as "KWAJ".
//!
//! Used for MS-DOS/Windows setup in the MS-DOS 6/Windows 3 era. The canonical extractor is the
//! 16-bit `EXPAND.EXE`.
//!
//! Files start with a `b"KWAJ\x88\xF0\x27\xD1"` magic and support five storage formats.


mod lzh;
mod sz;


use std::io::{Read, Write};

use crate::error::DecompressionError;


pub(crate) fn decompress<R: Read, W: Write>(compressed_reader: &mut R, decompressed_writer: &mut W) -> Result<(), DecompressionError> {
    // assuming we have already read the b"KWAJ\x88\xF0\x27\xD1" magic

    // read the compression type byte
    let mut compression_type_buf = [0u8];
    compressed_reader.read_exact(&mut compression_type_buf)?;

    // read the compression data offset
    let mut data_offset_buf = [0u8; 2];
    compressed_reader.read_exact(&mut data_offset_buf)?;
    let data_offset = u16::from_be_bytes(data_offset_buf);

    // get to the beginning of the compressed data
    const HEADER_ALREADY_READ: u16 = 8 + 1 + 2;
    if data_offset < HEADER_ALREADY_READ {
        // the data starts somewhere within the header?!
        return Err(DecompressionError::DataOffsetWithinHeader);
    }
    let eat_how_many_bytes = data_offset - (HEADER_ALREADY_READ);
    let mut wastebin = vec![0u8; eat_how_many_bytes.into()];
    compressed_reader.read_exact(&mut wastebin)?;

    match compression_type_buf[0] {
        0x00|0x01 => {
            // no compression
            // 0x01: additionally masked XOR 0xFF
            // why even bother, lol
            let mut buf = [0; 4*1024];
            loop {
                let bytes_read = compressed_reader.read(&mut buf)?;
                if bytes_read == 0 {
                    // enough
                    break;
                }

                if compression_type_buf[0] == 0x01 {
                    // unmask
                    for b in &mut buf[..bytes_read] {
                        *b ^= 0xFF;
                    }
                }

                decompressed_writer.write_all(&buf[..bytes_read])?;
            }
        },
        0x02 => {
            // "SZ" (not "SZDD")
            crate::kwaj::sz::decompress(compressed_reader, decompressed_writer, false)?;
        },
        0x03 => {
            // LZH (Lempel-Ziv + Huffman) by Jeff Johnson
            crate::kwaj::lzh::decompress(compressed_reader, decompressed_writer)?;
        },
        0x04 => {
            // MS-ZIP (CAB-like)
            todo!();
        },
        _ => return Err(DecompressionError::UnknownCompressionMethod),
    }

    Ok(())
}
