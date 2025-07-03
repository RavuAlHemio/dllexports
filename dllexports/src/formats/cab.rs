use std::collections::BTreeMap;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use expandms::cab::{CabData, CabFolder, CabHeader, CompressionType, FileInCab, FileInCabAttributes, FolderIndex};
use expandms::inflate::Inflater;
use expandms::lzx::LzxDecompressor;
use expandms::ring_buffer::RingBuffer;
use expandms::DecompressionError;
use tracing::debug;

use crate::data_mgmt::MultiFileContainer;


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct Cabinet {
    bytes: Vec<u8>,
    header: CabHeader,
    folders: Vec<CabFolder>,
    folder_data: Vec<Vec<CabData>>,
    files: Vec<FileInCab>,
    path_to_index: BTreeMap<PathBuf, usize>,
}
impl Cabinet {
    pub fn new(bytes: &[u8]) -> Result<Self, crate::data_mgmt::Error> {
        let mut reader = Cursor::new(bytes);
        let header = CabHeader::read(&mut reader)?;

        let mut folders = Vec::with_capacity(header.folder_count.into());
        for _ in 0..header.folder_count {
            let folder = CabFolder::read(&mut reader, &header)?;
            folders.push(folder);
        }

        let mut files = Vec::with_capacity(header.file_count.into());
        for _ in 0..header.file_count {
            let file = FileInCab::read(&mut reader)?;
            files.push(file);
        }

        let mut folder_data = Vec::with_capacity(folders.len());
        for folder in &folders {
            reader.seek(SeekFrom::Start(folder.start_offset.into()))?;
            let mut data_vec = Vec::with_capacity(folder.data_count.into());
            for _ in 0..folder.data_count {
                let data = CabData::read(&mut reader, &header)?;
                let compressed_byte_count = data.compressed_byte_count;
                data_vec.push(data);

                // skip the compressed data
                reader.seek(SeekFrom::Current(compressed_byte_count.into()))?;
            }
            folder_data.push(data_vec);
        }

        let mut path_to_index = BTreeMap::new();
        for (index, file) in files.iter().enumerate() {
            let path_string = if file.attributes.contains(FileInCabAttributes::UTF8_NAME) {
                // convert from UTF-8 bytes to PathBuf
                String::from_utf8(file.name.clone())
                    .map_err(|e| crate::data_mgmt::Error::InvalidUtf8FileName(e.into_bytes()))?
            } else {
                // locale-specific encoding, great
                // just do the naive ISO 8859-1 thing
                file.name.iter()
                    .map(|b| char::from_u32((*b).into()).unwrap())
                    .collect()
            };
            let path = PathBuf::from(path_string);
            path_to_index.insert(path, index);
        }

        Ok(Self {
            bytes: bytes.to_vec(),
            header,
            folders,
            folder_data,
            files,
            path_to_index,
        })
    }
}
impl MultiFileContainer for Cabinet {
    fn list_files(&self) -> Result<Vec<PathBuf>, crate::data_mgmt::Error> {
        let files = self.path_to_index.keys()
            .map(|k| k.clone())
            .collect();
        Ok(files)
    }

    fn read_file(&self, file_path: &Path) -> Result<Vec<u8>, crate::data_mgmt::Error> {
        let Some(&index) = self.path_to_index.get(file_path) else {
            return Err(crate::data_mgmt::Error::FileNotFound(file_path.to_owned()));
        };
        let file = &self.files[index];
        let folder_index: usize = match file.folder_index {
            FolderIndex::RegularIndex(i)
                => i.into(),
            FolderIndex::ContinuedFromPrevious|FolderIndex::ContinuedToNext
            |FolderIndex::ContinuedPreviousAndNext
                => return Err(crate::data_mgmt::Error::SpannedFile),
        };
        let folder = &self.folders[folder_index];
        match folder.compression_type {
            CompressionType::NoCompression => {
                let mut collector = FileCollector::new(
                    file.uncompressed_offset_in_folder.try_into().unwrap(),
                    file.uncompressed_size_bytes.try_into().unwrap(),
                );
                for data_block in &self.folder_data[folder_index] {
                    // make a "decompressor"
                    let data_slice_length = usize::from(data_block.compressed_byte_count);
                    let slice = &self.bytes[data_block.data_offset..data_block.data_offset+data_slice_length];
                    let cursor = Cursor::new(slice);
                    let mut block_decompressor = FileDecompressor::NoCompression(cursor);
                    loop {
                        match collector.read(&mut block_decompressor)? {
                            FileReadStatus::ReadProgress => {
                                // good, keep going
                            },
                            FileReadStatus::FileComplete => {
                                // wahey!
                                return Ok(collector.decompressed_buffer);
                            },
                            FileReadStatus::DecompressorExhausted => {
                                // okay, consume the next data block
                                break;
                            },
                        }
                    }
                }

                // if we land here, the last status was DecompressorExhausted
                // => the file is at an offset not actually in the CAB file
                return Err(crate::data_mgmt::Error::Io(io::ErrorKind::UnexpectedEof.into()));
            },
            CompressionType::MsZip => {
                let mut collector = FileCollector::new(
                    file.uncompressed_offset_in_folder.try_into().unwrap(),
                    file.uncompressed_size_bytes.try_into().unwrap(),
                );
                let mut lookback = RingBuffer::new(0x00);
                for data_block in &self.folder_data[folder_index] {
                    // make a decompressor

                    let data_slice_length = usize::from(data_block.compressed_byte_count);
                    let slice = &self.bytes[data_block.data_offset..data_block.data_offset+data_slice_length];
                    let mut cursor = Cursor::new(slice);

                    // check for MSZIP header
                    let mut mszip_header = [0u8; 2];
                    cursor.read_exact(&mut mszip_header)?;
                    if &mszip_header != b"CK" {
                        return Err(crate::data_mgmt::Error::Decompression(DecompressionError::UnknownCompressionMethod));
                    }

                    let mut inflater = Inflater::new(&mut cursor);
                    inflater.set_lookback(lookback);
                    let mut block_decompressor = FileDecompressor::MsZip {
                        inflater,
                        last_block_read: false,
                    };
                    loop {
                        match collector.read(&mut block_decompressor)? {
                            FileReadStatus::ReadProgress => {},
                            FileReadStatus::FileComplete => return Ok(collector.decompressed_buffer),
                            FileReadStatus::DecompressorExhausted => break,
                        }
                    }
                    // we will need this for the next pass
                    lookback = block_decompressor.inflater().unwrap().lookback().clone();
                }

                // see above
                return Err(crate::data_mgmt::Error::Io(io::ErrorKind::UnexpectedEof.into()));
            },
            CompressionType::Quantum
                => return Err(crate::data_mgmt::Error::Decompression(DecompressionError::UnknownCompressionMethod)),
            CompressionType::Lzx => {
                let mut collector = FileCollector::new(
                    file.uncompressed_offset_in_folder.try_into().unwrap(),
                    file.uncompressed_size_bytes.try_into().unwrap(),
                );

                // make the magic reader that reads all blocks
                let mut all_blocks_reader = CabFolderReader::new(
                    Cursor::new(self.bytes.as_slice()),
                    self.folder_data[folder_index].clone(),
                );

                // make a decompressor
                let window_size_exponent = (self.folders[folder_index].compression_parameters >> 4) & 0xFF;
                let decompressor = LzxDecompressor::new(
                    &mut all_blocks_reader,
                    window_size_exponent.into(),
                )?;

                let total_uncompressed_length = self.folder_data[folder_index]
                    .iter()
                    .map(|fd| usize::from(fd.uncompressed_byte_count))
                    .sum();

                let mut block_decompressor = FileDecompressor::Lzx {
                    decompressor,
                    uncompressed_length: total_uncompressed_length,
                };
                loop {
                    match collector.read(&mut block_decompressor)? {
                        FileReadStatus::ReadProgress => {},
                        FileReadStatus::FileComplete => return Ok(collector.decompressed_buffer),
                        FileReadStatus::DecompressorExhausted => break,
                    }
                }

                // see above
                return Err(crate::data_mgmt::Error::Io(io::ErrorKind::UnexpectedEof.into()));
            },
            CompressionType::Other(_)
                => return Err(crate::data_mgmt::Error::Decompression(DecompressionError::UnknownCompressionMethod)),
        }
    }
}

struct FileCollector {
    pub file_start: usize,
    pub file_length: usize,
    pub bytes_dropped: usize,
    pub decompressed_buffer: Vec<u8>,
}
impl FileCollector {
    pub fn new(file_start: usize, file_length: usize) -> Self {
        Self {
            file_start,
            file_length,
            bytes_dropped: 0,
            decompressed_buffer: Vec::new(),
        }
    }

    pub fn read<'r>(&mut self, decompressor: &mut FileDecompressor<'r>) -> Result<FileReadStatus, crate::data_mgmt::Error> {
        // are we done?
        if self.decompressed_buffer.len() >= self.file_length {
            // cut down to size if needed
            if self.decompressed_buffer.len() > self.file_length {
                self.decompressed_buffer.drain(self.file_length..);
            }
            return Ok(FileReadStatus::FileComplete);
        }

        // read the next chunk
        let decompressed = decompressor.decompress_one()?;
        if decompressed.len() == 0 {
            // crud
            return Ok(FileReadStatus::DecompressorExhausted);
        }
        self.decompressed_buffer.extend_from_slice(&decompressed);
        if self.file_start > self.bytes_dropped {
            // we need to cut a piece from the beginning
            let additionally_drop = self.file_start - self.bytes_dropped;
            let actually_additionally_drop = additionally_drop.min(self.decompressed_buffer.len());
            self.decompressed_buffer.drain(..actually_additionally_drop);
            self.bytes_dropped += actually_additionally_drop;
        }

        // ask to be called again
        Ok(FileReadStatus::ReadProgress)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum FileReadStatus {
    ReadProgress,
    FileComplete,
    DecompressorExhausted,
}

enum FileDecompressor<'r> {
    NoCompression(Cursor<&'r [u8]>),
    MsZip {
        inflater: Inflater<'r, Cursor<&'r [u8]>>,
        last_block_read: bool,
    },
    Lzx {
        decompressor: LzxDecompressor<'r, CabFolderReader<Cursor<&'r [u8]>>>,
        uncompressed_length: usize,
    },
}
impl<'r> FileDecompressor<'r> {
    pub fn decompress_one(&mut self) -> Result<Vec<u8>, crate::data_mgmt::Error> {
        match self {
            Self::NoCompression(c) => {
                let mut buf = Vec::new();
                c.read_to_end(&mut buf)?;
                Ok(buf)
            },
            Self::MsZip { inflater, last_block_read } => {
                let mut buf = Vec::new();
                if !*last_block_read {
                    let is_last = inflater.inflate_block(&mut buf)?;
                    if is_last {
                        *last_block_read = true;
                    }
                }
                Ok(buf)
            },
            Self::Lzx { decompressor, uncompressed_length } => {
                let mut buf = Vec::new();
                while buf.len() < *uncompressed_length {
                    debug!("decompressing next LZX block ({}/{})", buf.len(), uncompressed_length);
                    decompressor.decompress_block(&mut buf)?;
                }
                Ok(buf)
            },
        }
    }

    pub fn inflater(&self) -> Option<&Inflater<'r, Cursor<&'r [u8]>>> {
        match self {
            Self::NoCompression(_) => None,
            Self::MsZip { inflater, .. } => Some(inflater),
            Self::Lzx { .. } => None,
        }
    }
}

pub(crate) struct CabFolderReader<R: Read + Seek> {
    reader: R,
    folder_data: Vec<CabData>,

    data_index: usize,
    data_pos: u16,
}
impl<R: Read + Seek> CabFolderReader<R> {
    pub fn new(reader: R, folder_data: Vec<CabData>) -> Self {
        Self {
            reader,
            folder_data,

            data_index: 0,
            data_pos: 0,
        }
    }
}
impl<R: Read + Seek> Read for CabFolderReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.len() == 0 {
            return Ok(0);
        }

        loop {
            if self.data_index >= self.folder_data.len() {
                // EOF
                return Ok(0);
            }

            let data_block = &self.folder_data[self.data_index];
            if self.data_pos >= data_block.compressed_byte_count {
                // advance to the next block
                self.data_index += 1;
                self.data_pos = 0;
                continue;
            }
            if self.data_pos == 0 {
                // seek to beginning of block
                self.reader.seek(SeekFrom::Start(u64::try_from(data_block.data_offset).unwrap()))?;
            }

            // read either the rest of the current block or enough to fill the buffer
            // (whichever is shorter)
            let rest_of_block_len = data_block.compressed_byte_count - self.data_pos;
            let read_count = usize::from(rest_of_block_len)
                .min(buf.len());
            let actually_read = self.reader.read(&mut buf[..read_count])?;
            assert!(actually_read <= read_count);
            self.data_pos += u16::try_from(actually_read).unwrap();

            return Ok(actually_read);
        }
    }
}
