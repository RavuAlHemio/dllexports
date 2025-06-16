use std::collections::BTreeMap;
use std::io::{Cursor, Error, Read, Seek, SeekFrom};
use std::path::PathBuf;

use expandms::iso9660::{DirectoryRecord, FileFlags, VolumeDescriptor};

use crate::data_mgmt::MultiFileContainer;


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Cdrom {
    pub data: Vec<u8>,
    pub path_to_entry: BTreeMap<PathBuf, FileEntry>,
}
impl Cdrom {
    fn new_from_data(data: &[u8], is_high_sierra: bool) -> Result<Self, Error> {
        // read basic volume descriptor
        let mut reader = Cursor::new(data);
        reader.seek(SeekFrom::Start(0x8000))?;
        let vd = VolumeDescriptor::read(&mut reader, is_high_sierra)?;
        let block_size = vd.logical_block_size.little_endian;

        let root_directory_location = u64::from(block_size) * u64::from(vd.root_directory_record.extent_location.little_endian);
        let mut directory_stack = vec![Directory {
            path: PathBuf::new(),
            offset: root_directory_location,
            size: vd.root_directory_record.data_length.little_endian.try_into().unwrap(),
        }];
        let mut path_to_entry = BTreeMap::new();
        while let Some(directory) = directory_stack.pop() {
            reader.seek(SeekFrom::Start(directory.offset))?;
            let mut directory_bytes = vec![0u8; directory.size];
            reader.read_exact(&mut directory_bytes)?;
            let mut pos = 0;

            while pos < directory_bytes.len() {
                let length: u8 = directory_bytes[pos];
                pos += 1;
                if length == 0 {
                    // hmm, maybe check the next logical sector?
                    if pos % 0x800 == 1 {
                        // we already are at the logical sector boundary and there's nothing there
                        break;
                    } else {
                        pos = pos + (0x800 - pos % 0x800);
                        continue;
                    }
                }

                let dr = DirectoryRecord::read_after_length(&directory_bytes, &mut pos, length, is_high_sierra);
                let filename: String = dr.file_identifier.iter()
                    .map(|b| char::from_u32((*b).into()).unwrap())
                    .collect();
                if dr.file_flags.contains(FileFlags::DIRECTORY) && (filename == "\u{00}" || filename == "\u{01}") {
                    // reference to root or parent
                    continue;
                }
                let mut full_path = directory.path.clone();
                full_path.push(&filename);

                let offset = u64::from(block_size) * u64::from(dr.extent_location.little_endian);
                let size = usize::try_from(dr.data_length.little_endian).unwrap();

                if dr.file_flags.contains(FileFlags::DIRECTORY) {
                    directory_stack.push(Directory {
                        path: full_path,
                        offset,
                        size,
                    });
                } else {
                    path_to_entry.insert(
                        full_path,
                        FileEntry {
                            offset,
                            size,
                        },
                    );
                }
            }
        }
        Ok(Cdrom {
            data: data.to_vec(),
            path_to_entry,
        })
    }

    pub fn new_from_iso9660_data(data: &[u8]) -> Result<Self, Error> {
        Self::new_from_data(data, false)
    }

    pub fn new_from_high_sierra_data(data: &[u8]) -> Result<Self, Error> {
        Self::new_from_data(data, true)
    }
}
impl MultiFileContainer for Cdrom {
    fn list_files(&self) -> Result<Vec<PathBuf>, crate::data_mgmt::Error> {
        let files = self.path_to_entry
            .keys()
            .map(|p| p.clone())
            .collect();
        Ok(files)
    }

    fn read_file(&self, file_path: &std::path::Path) -> Result<Vec<u8>, crate::data_mgmt::Error> {
        let entry = self.path_to_entry.get(file_path)
            .ok_or_else(|| crate::data_mgmt::Error::FileNotFound(file_path.to_owned()))?;
        let mut cursor = Cursor::new(&self.data);
        cursor.seek(SeekFrom::Start(entry.offset))?;
        let mut buf = vec![0u8; entry.size];
        cursor.read_exact(&mut buf)?;
        Ok(buf)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Directory {
    pub path: PathBuf,
    pub offset: u64,
    pub size: usize,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FileEntry {
    pub offset: u64,
    pub size: usize,
}
