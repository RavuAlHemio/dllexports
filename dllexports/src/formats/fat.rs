use std::collections::BTreeMap;
use std::fmt;
use std::io::{Cursor, ErrorKind, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use expandms::fat::{
    AllocationTable, Attributes, DirectoryEntry, DIRECTORY_ENTRY_SIZE_BYTES, FatEntry, FatHeader,
    read_cluster_chain_into, RootDirectoryLocation,
};
use tracing::debug;

use crate::data_mgmt::{Error, MultiFileContainer};


#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FatFileSystem {
    data: Vec<u8>,
    header: FatHeader,
    fat: AllocationTable,
    file_path_to_first_cluster: BTreeMap<PathBuf, u32>,
}
impl FatFileSystem {
    pub fn new(data: Vec<u8>) -> Result<Self, Error> {
        let mut cursor = Cursor::new(&data);

        // read header
        let header = FatHeader::read(&mut cursor)?;
        if header.fat_bytes() > data.len() {
            // yeah, that's an invalid header
            debug!("header claims FAT byte count is greater than fits into file");
            return Err(Error::Io(ErrorKind::InvalidData.into()));
        }

        // skip over reserved sectors
        let reserved_bytes = u64::from(header.reserved_sector_count) * u64::from(header.bytes_per_sector);
        cursor.seek(SeekFrom::Start(reserved_bytes))?;

        // read FAT
        let fat = AllocationTable::read(
            &mut cursor,
            header.variant(),
            header.fat_bytes(),
        )?;

        let Some(first_fat_entry) = fat.entries.get(0) else {
            debug!("FAT is missing entry at index 0");
            return Err(Error::Io(ErrorKind::InvalidData.into()));
        };
        match first_fat_entry {
            FatEntry::MediaType(_) => {},
            _ => {
                // not actually FAT
                debug!("FAT entry at index 0 is not a media type entry");
                return Err(Error::Io(ErrorKind::InvalidData.into()));
            },
        }

        // find and read root directory
        let root_directory_bytes = match header.root_directory_location {
            RootDirectoryLocation::Sector(sector_index) => {
                let entries_to_read: usize = header.max_root_dir_entries.into();
                let bytes_to_read = entries_to_read * DIRECTORY_ENTRY_SIZE_BYTES;
                let mut output = Vec::with_capacity(bytes_to_read);
                expandms::fat::read_sector_into(
                    &mut cursor,
                    &header,
                    sector_index,
                    &mut output,
                )?;
                if output.len() < bytes_to_read {
                    // root directory spans more than one sector; read the rest
                    let remaining_bytes = output.len();
                    cursor.read_exact(&mut output[remaining_bytes..])?;
                }
                output
            },
            RootDirectoryLocation::Cluster(first_cluster_index) => {
                let mut output = Vec::new();
                expandms::fat::read_cluster_chain_into(
                    &mut cursor,
                    &header,
                    &fat,
                    first_cluster_index,
                    &mut output,
                )?;
                output
            },
        };

        // recursively add all files in the file system
        let mut me = Self {
            data: data.clone(),
            header,
            fat,
            file_path_to_first_cluster: BTreeMap::new(),
        };
        me.process_directory(Path::new(&""), &mut cursor, &root_directory_bytes)?;

        Ok(me)
    }

    fn process_directory<R: Read + Seek>(&mut self, path_prefix: &Path, reader: &mut R, directory_bytes: &[u8]) -> Result<(), Error> {
        // run through the directory
        let mut cursor = Cursor::new(directory_bytes);
        loop {
            let entree = match DirectoryEntry::read(&mut cursor, self.header.variant()) {
                Ok(e) => e,
                Err(e) => {
                    if e.kind() == ErrorKind::UnexpectedEof {
                        break;
                    } else {
                        return Err(Error::Io(e));
                    }
                },
            };

            if entree.file_name[0] == 0x00 {
                // no more entries in this directory
                break;
            } else if entree.file_name[0] == 0xE5 {
                // this is not an entry but there might be more
                continue;
            }

            if entree.file_name.as_ref() == b"..      " && entree.extension.as_ref() == b"   " {
                // parent directory
                continue;
            } else if entree.file_name.as_ref() == b".       " && entree.extension.as_ref() == b"   " {
                // the directory itself
                continue;
            }

            if entree.attributes.contains(Attributes::VOLUME_LABEL) {
                // FIXME: VFAT long file names?
                continue;
            }

            let mut name = String::with_capacity(12);
            for &b in entree.file_name.as_ref() {
                name.push(char::from_u32(b.into()).unwrap());
            }
            while name.ends_with(" ") {
                name.pop();
            }
            name.push('.');
            for &b in entree.extension.as_ref() {
                name.push(char::from_u32(b.into()).unwrap());
            }
            while name.ends_with(" ") {
                name.pop();
            }

            let mut subpath = path_prefix.to_owned();
            subpath.push(&name);

            if entree.attributes.contains(Attributes::SUBDIRECTORY) {
                // curses! recursion!
                let mut subdir_data = Vec::new();
                read_cluster_chain_into(
                    reader,
                    &self.header,
                    &self.fat,
                    entree.first_cluster_number,
                    &mut subdir_data,
                )?;
                self.process_directory(
                    &subpath,
                    reader,
                    &subdir_data,
                )?;
            } else {
                // remember this one
                self.file_path_to_first_cluster.insert(subpath, entree.first_cluster_number);
            }
        }
        Ok(())
    }
}
impl fmt::Debug for FatFileSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FatFileSystem")
            .field("data", &"[removed]")
            .field("header", &self.header)
            .field("fat", &self.fat)
            .field("file_path_to_first_cluster", &self.file_path_to_first_cluster)
            .finish()
    }
}
impl MultiFileContainer for FatFileSystem {
    fn list_files(&self) -> Result<Vec<PathBuf>, Error> {
        let mut ret = Vec::with_capacity(self.file_path_to_first_cluster.len());
        for path in self.file_path_to_first_cluster.keys() {
            ret.push(path.clone());
        }
        Ok(ret)
    }

    fn read_file(&self, file_path: &std::path::Path) -> Result<Vec<u8>, Error> {
        let first_cluster_index = *self.file_path_to_first_cluster
            .get(file_path)
            .ok_or_else(|| Error::FileNotFound(file_path.to_owned()))?;
        let mut cursor = Cursor::new(&self.data);
        let mut data = Vec::new();
        expandms::fat::read_cluster_chain_into(
            &mut cursor,
            &self.header,
            &self.fat,
            first_cluster_index,
            &mut data,
        )?;
        Ok(data)
    }
}
