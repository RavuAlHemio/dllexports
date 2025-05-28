use std::io::{Read, Write};

use crate::error::DecompressionError;
use crate::ring_buffer::RingBuffer;


const RING_BUFFER_SIZE: usize = 4096;

pub(crate) fn decompress<R: Read, W: Write>(
    compressed_reader: &mut R,
    decompressed_writer: &mut W,
    szdd: bool,
) -> Result<(), DecompressionError> {
    let mut ring_buffer: RingBuffer<u8, RING_BUFFER_SIZE> = RingBuffer::new(0x20);
    ring_buffer.set_position(RING_BUFFER_SIZE - if szdd { 16 } else { 18 });

    loop {
        let mut control_buf = [0u8];
        let bytes_read = compressed_reader.read(&mut control_buf)?;
        debug_assert!(bytes_read < 2);
        if bytes_read == 0 {
            break;
        }

        for control_bit in 0..8 {
            if control_buf[0] & (1 << control_bit) != 0 {
                // literal
                let mut byte_buf = [0u8];
                compressed_reader.read_exact(&mut byte_buf)?;
                ring_buffer.push(byte_buf[0]);
                decompressed_writer.write_all(&byte_buf)?;
            } else {
                // match
                // read two bytes:
                // b0 = P7 P6 P5 P4 P3 P2 P1 P0
                // b1 = Pb Pa P9 P8 L3 L2 L1 L0
                // where P = match position, L = match length
                let mut match_param_buf = [0u8; 2];
                compressed_reader.read_exact(&mut match_param_buf)?;
                let mut match_position
                    = usize::from(match_param_buf[0])
                    | (usize::from(match_param_buf[1]) & 0x00F0) << 4;
                let match_length
                    = usize::from(match_param_buf[1]) & 0x000F;
                let mut byte_buf = [0u8];
                for _ in 0..match_length {
                    byte_buf[0] = ring_buffer.as_slice()[match_position];
                    match_position = (match_position + 1) % RING_BUFFER_SIZE;
                    decompressed_writer.write_all(&byte_buf)?;
                    ring_buffer.push(byte_buf[0]);
                }
            }
        }
    }

    Ok(())
}
