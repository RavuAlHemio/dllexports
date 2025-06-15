use std::collections::BTreeMap;
use std::path::PathBuf;


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Cdrom {
    pub path_to_entry: BTreeMap<PathBuf, FileEntry>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FileEntry {
    pub offset: u64,
    pub size: usize,
}
