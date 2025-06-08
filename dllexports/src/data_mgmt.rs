use std::path::{Path, PathBuf};


/// A sequence of paths, possibly through multiple file systems.
///
/// The first part is the path through the host system's file system. Any subsequent part identifies
/// a file in a container; if the part is empty, this refers to the single file in a single-file
/// container.
///
/// For example, if we have a compressed `user.exe` within the KWAJ container `user.ex_` within the
/// FAT12 image `disk01.img` on the host file system, then `["disk01.img", "user.ex_"]` refers to
/// `user.ex_` while `["disk01.img", "user.ex_", ""]` refers to `user.exe` (since `user.ex_` only
/// contains one file and generally doesn't hint at the file's actual name).
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PathSequence {
    parts: Vec<PathBuf>,
}

/// A container file that contains multiple files.
///
/// This can be a compression-enabled archive format like PKZIP, an uncompressed archive format like
/// TAR, or a file system image like FAT or ISO9660.
pub trait MultiFileContainer {
    fn list_files(&self) -> Result<Vec<PathBuf>, Error>;
    fn read_file(&self, file_path: &Path) -> Result<Vec<u8>, Error>;
}

/// A container file that contains a single file.
///
/// Generally a single-file compression format such as gzip or KWAJ.
pub trait SingleFileContainer {
    fn read_file(&self) -> Result<Vec<u8>, Error>;
}

/// A file that exports symbols.
///
/// This is generally a dynamic-link library format like NE or PE.
pub trait SymbolExporter {
    fn read_symbols(&self) -> Result<Vec<Symbol>, Error>;
}

/// A file with its contents interpreted.
pub enum IdentifiedFile {
    MultiFileContainer(Box<dyn MultiFileContainer>),
    SingleFileContainer(Box<dyn SingleFileContainer>),
    SymbolExporter(Box<dyn SymbolExporter>),
    Unidentified,
}

/// A single exported symbol.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Symbol {
    ByName { name: String },
    ByOrdinal { ordinal: u32 },
    ByNameAndOrdinal { name: String, ordinal: u32 },
}

/// Sometimes things go wrong.
#[derive(Debug)]
pub enum Error {
}
