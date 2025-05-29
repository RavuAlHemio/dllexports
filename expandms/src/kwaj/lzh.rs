/// Decompressor for the Lempel-Ziv + Huffman combo created by Jeff Johnson.


use std::io::{self, Read, Write};

use crate::error::DecompressionError;
use crate::huff::HuffmanTree;
use crate::io_util::BitReader;
use crate::ring_buffer::RingBuffer;


fn create_huffman_tree<R: Read>(compressed_reader: &mut BitReader<&mut R, true>, symbol_count: usize, encoding_type: u8) -> Result<HuffmanTree<u8>, DecompressionError> {
    let symbol_lengths = match encoding_type {
        0 => {
            // same length for all symbols, depending on symbol count (log_2 n)
            match symbol_count {
                16 => vec![4; 16],
                32 => vec![5; 32],
                64 => vec![6; 64],
                256 => vec![8; 256],
                _ => return Err(DecompressionError::UnexpectedHuffmanSymbolCount { symbol_count }),
            }
        },
        1 => {
            // run-length-encoded list
            let mut symbol_lengths: Vec<usize> = Vec::with_capacity(symbol_count);

            // 4 bits contain the first symbol length
            let first_length = compressed_reader.read_u4()?;
            symbol_lengths.push(first_length.into());

            // now loop
            for _ in 1..symbol_count {
                let not_same_as_prev = compressed_reader.read_bit_strict()?;
                if !not_same_as_prev {
                    let prev = *symbol_lengths.last().unwrap();
                    symbol_lengths.push(prev);
                    continue;
                }
                let not_same_as_prev_plus_1 = compressed_reader.read_bit_strict()?;
                if !not_same_as_prev_plus_1 {
                    let prev = *symbol_lengths.last().unwrap();
                    symbol_lengths.push(prev + 1);
                    continue;
                }
                let next_length = compressed_reader.read_u4()?;
                symbol_lengths.push(next_length.into());
            }
            symbol_lengths
        },
        2 => {
            // run-length delta encoding
            let mut symbol_lengths: Vec<usize> = Vec::with_capacity(symbol_count);

            // 4 bits contain the first symbol length
            let first_length = compressed_reader.read_u4()?;
            symbol_lengths.push(first_length.into());

            // now loop
            for _ in 1..symbol_count {
                // read selector
                let selector = compressed_reader.read_u2()?;
                match selector {
                    0|1|2 => {
                        // value +/-1 of the previous
                        let previous_length = *symbol_lengths.last().unwrap();
                        if previous_length == 0 && selector == 0 {
                            // can't do 0 - 1
                            return Err(DecompressionError::RelativeValueUnderflow);
                        }
                        let next_length = previous_length + usize::from(selector) - 1;
                        symbol_lengths.push(next_length);
                    },
                    3 => {
                        // completely new value
                        let next_length = compressed_reader.read_u4()?;
                        symbol_lengths.push(next_length.into());
                    },
                    _ => unreachable!(),
                }
            }
            symbol_lengths
        },
        3 => {
            // raw encoding, 4 bits per symbol
            let mut symbol_lengths: Vec<usize> = Vec::with_capacity(symbol_count);
            for _ in 0..symbol_count {
                let symbol_length = compressed_reader.read_u4()?;
                symbol_lengths.push(symbol_length.into());
            }
            symbol_lengths
        },
        other => return Err(DecompressionError::UnknownHuffmanTreeEncoding { encoding: other }),
    };
    assert!(symbol_lengths.len() == symbol_count);

    let tree = HuffmanTree::new_canonical(&symbol_lengths)?;
    Ok(tree)
}


fn ignore_eof(operation_result: Result<(), io::Error>) -> Result<(), DecompressionError> {
    match operation_result {
        Ok(()) => Ok(()),
        Err(e) => {
            if e.kind() == io::ErrorKind::UnexpectedEof {
                Ok(())
            } else {
                Err(DecompressionError::Io(e))
            }
        },
    }
}


pub(crate) fn decompress<R: Read, W: Write>(compressed_reader: &mut R, decompressed_writer: &mut W) -> Result<(), DecompressionError> {
    // assuming we have already read the b"KWAJ\x88\xF0\x27\xD1" magic,
    // the compression type byte 0x03, the compressed data offset word
    // and all the data preceding the compressed data offset

    // read 3 bytes containing encoding types
    let mut encoding_types = [0u8; 3];
    compressed_reader.read_exact(&mut encoding_types)?;

    let match_run_lengths_encoding_type = (encoding_types[0] >> 4) & 0x0F;
    let match_run_lengths_after_short_encoding_type = (encoding_types[0] >> 0) & 0x0F;
    let literal_run_lengths_encoding_type = (encoding_types[1] >> 4) & 0x0F;
    let offset_tops_encoding_type = (encoding_types[1] >> 0) & 0x0F;
    let literals_encoding_type = (encoding_types[2] >> 4) & 0x0F;
    // bottom half of encoding_types[2] is padding

    // wrap reader into bit reader
    let mut bit_reader = BitReader::new(compressed_reader);

    // build the tables
    let match_run_lengths = create_huffman_tree(&mut bit_reader, 16, match_run_lengths_encoding_type)?;
    let match_run_lengths_after_short = create_huffman_tree(&mut bit_reader, 16, match_run_lengths_after_short_encoding_type)?;
    let literal_run_lengths = create_huffman_tree(&mut bit_reader, 32, literal_run_lengths_encoding_type)?;
    let offset_tops = create_huffman_tree(&mut bit_reader, 64, offset_tops_encoding_type)?;
    let literals = create_huffman_tree(&mut bit_reader, 256, literals_encoding_type)?;

    const RING_BUFFER_SIZE: usize = 4096;
    let mut ring_buffer: RingBuffer<u8, RING_BUFFER_SIZE> = RingBuffer::new(0x20);
    ring_buffer.set_position(RING_BUFFER_SIZE - 17);
    let mut current_lookup = &match_run_lengths;
    while let Some(&code) = current_lookup.decode_one_from_bit_reader(&mut bit_reader)? {
        // note that there is no end symbol, so if we run out of bits midway, we just exit
        println!("pos is {}", ring_buffer.position());

        if code > 0 {
            // match
            let match_length_u8 = code + 2;
            let match_length: usize = match_length_u8.into();
            let offset_top = match offset_tops.decode_one_from_bit_reader(&mut bit_reader) {
                Ok(Some(ot)) => *ot,
                Ok(None) => return Ok(()),
                Err(e) => return ignore_eof(Err(e)),
            };
            let offset_bottom = match bit_reader.read_u6() {
                Ok(ob) => ob,
                Err(e) => return ignore_eof(Err(e)),
            };
            let offset =
                (usize::from(offset_top) << 6)
                | usize::from(offset_bottom);
            println!("offset is {}, len is {}", match_length, offset);
            let mut match_offset = (ring_buffer.position() + RING_BUFFER_SIZE - offset) % RING_BUFFER_SIZE;

            // copy match_length bytes starting at match_offset
            // both into the output and into the current ring buffer position
            for _ in 0..match_length {
                let match_buf = [ring_buffer.as_slice()[match_offset]];
                println!("  I got {:02x}", match_buf[0]);
                decompressed_writer.write_all(&match_buf)?;
                ring_buffer.extend(match_buf);
                match_offset = (match_offset + 1) % RING_BUFFER_SIZE;
            }
            current_lookup = &match_run_lengths;
        } else {
            // run of literals
            let literal_length = match literal_run_lengths.decode_one_from_bit_reader(&mut bit_reader) {
                Ok(Some(ll)) => *ll,
                Ok(None) => return Ok(()),
                Err(e) => return ignore_eof(Err(e)),
            };
            println!("literal len is {}", literal_length);
            if literal_length != 31 {
                current_lookup = &match_run_lengths_after_short;
            }
            for _ in 0..literal_length {
                let byte = match literals.decode_one_from_bit_reader(&mut bit_reader) {
                    Ok(Some(b)) => *b,
                    Ok(None) => return Ok(()),
                    Err(e) => return ignore_eof(Err(e)),
                };
                println!("  I got {:02x}", byte);
                decompressed_writer.write_all(&[byte])?;
                ring_buffer.push(byte);
            }
        }
    }

    Ok(())
}
