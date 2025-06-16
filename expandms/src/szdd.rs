use std::io::{Read, Write};

use tracing::debug;

use crate::error::DecompressionError;


const WINDOW_SIZE: usize = 4096;


pub fn decompress_szdd<R: Read, W: Write>(compressed_reader: &mut R, decompressed_writer: &mut W) -> Result<(), DecompressionError> {
    // assuming we have already read the b"SZDD\x88\xF0\x27\x33" magic
    let mut header = [0u8; 6];
    compressed_reader.read_exact(&mut header)?;
    if header[0] != b'A' {
        return Err(DecompressionError::UnknownCompressionMethod);
    }
    let decompressed_size = u32::from_le_bytes(header[2..6].try_into().unwrap());

    decompress_sz_generic(compressed_reader, decompressed_writer, decompressed_size, 16)
}


pub fn decompress_sz<R: Read, W: Write>(compressed_reader: &mut R, decompressed_writer: &mut W) -> Result<(), DecompressionError> {
    // assuming we have already read the b"SZ \x88\xF0\x27\x33\xD1" magic
    let mut header = [0u8; 4];
    compressed_reader.read_exact(&mut header)?;
    let decompressed_size = u32::from_le_bytes(header);

    decompress_sz_generic(compressed_reader, decompressed_writer, decompressed_size, 18)
}

fn decompress_sz_generic<R: Read, W: Write>(
    compressed_reader: &mut R,
    decompressed_writer: &mut W,
    decompressed_size: u32,
    initial_window_position_from_end: usize,
) -> Result<(), DecompressionError> {
    let mut window = [b' '; WINDOW_SIZE];
    let mut pos = window.len() - initial_window_position_from_end;
    let mut bytes_written = 0;

    loop {
        let mut control_byte_buf = [0u8];
        let bytes_read = compressed_reader.read(&mut control_byte_buf)?;
        if bytes_read == 0 {
            // EOF
            break;
        }
        let control_byte = control_byte_buf[0];

        if tracing::enabled!(tracing::Level::DEBUG) {
            let mut control_bits = [0u8; 8];
            for i in 0..8 {
                if control_byte & (1 << i) != 0 {
                    control_bits[i] = b'1';
                } else {
                    control_bits[i] = b'0';
                }
            }
            let s = std::str::from_utf8(&control_bits).unwrap();
            debug!("control bits: 0b{}", s);
        }

        for shift_count in 0..8 {
            if control_byte & (1 << shift_count) != 0 {
                // literal byte
                let mut lit_byte_buf = [0u8];
                compressed_reader.read_exact(&mut lit_byte_buf)?;

                decompressed_writer.write_all(&lit_byte_buf)?;
                bytes_written += 1;
                if bytes_written == decompressed_size {
                    return Ok(());
                }

                window[pos] = lit_byte_buf[0];
                pos = (pos + 1) % WINDOW_SIZE;
            } else {
                // Msb   Lsb   Msb   Lsb
                // |       |   |       |
                // pppp pppp | PPPP llll
                // => PPPP pppp pppp, (llll + 3)
                let mut match_info_buf = [0u8; 2];
                compressed_reader.read_exact(&mut match_info_buf)?;
                let mut match_position =
                    usize::from(match_info_buf[0])
                    | (usize::from(match_info_buf[1] & 0xF0) << 4);
                let match_length = usize::from(match_info_buf[1] & 0xF) + 3;
                debug!("match at {} for {}", match_position, match_length);

                for _ in 0..match_length {
                    let b = window[match_position];
                    match_position = (match_position + 1) % WINDOW_SIZE;

                    decompressed_writer.write_all(&[b])?;
                    bytes_written += 1;
                    if bytes_written == decompressed_size {
                        return Ok(());
                    }

                    window[pos] = b;
                    pos = (pos + 1) % WINDOW_SIZE;
                }
            }
        }
    }

    Ok(())
}
