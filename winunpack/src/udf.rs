use std::ffi::{CStr, CString, c_void};
use std::path::Path;
use std::ptr::null_mut;

use libcdio_sys::{
    udf_close, udf_dirent_free, udf_dirent_t, udf_fopen, udf_get_file_length, udf_get_filename, udf_get_root, udf_is_dir, udf_open, udf_opendir, udf_read_block, udf_readdir, udf_t
};


/// The length, in bytes, of one UDF block.
pub const BLOCK_LENGTH: usize = const {
    if std::mem::size_of::<u32>() > std::mem::size_of::<usize>() {
        if libcdio_sys::udf_enum1_t_UDF_BLOCKSIZE > (usize::MAX as u32) {
            panic!("UDF_BLOCKSIZE too large for usize");
        }
    }
    libcdio_sys::udf_enum1_t_UDF_BLOCKSIZE as usize
};


/// A Universal Disk Format file system, either on an actual disc or as an image file.
///
/// UDF, a profile of ISO/IEC 13346 and ECMA-167, is the successor format to ISO 9660. DVD-Video
/// and DVD-Audio discs must abide by this specification and many DVD-ROM discs do too. It is also
/// often used on rewritable optical media such as CD-RW and DVD-RW, while ISO 9660 remains the
/// format of choice for CD-ROM and its write-once-read-many variant, CD-R.
pub struct Udf {
    handle: *mut udf_t,
}
impl Udf {
    /// Opens a UDF file system that is present at the given path.
    ///
    /// Returns `None` if opening fails. Panics if `path` contains NUL bytes.
    pub fn open(path: &Path) -> Option<Udf> {
        let path_c_string = CString::new(path.as_os_str().as_encoded_bytes())
            .expect("UDF path contains NULs");
        let handle = unsafe { udf_open(path_c_string.as_ptr()) };
        if handle.is_null() {
            None
        } else {
            Some(Self {
                handle,
            })
        }
    }

    /// Closes the UDF file system and returns whether closing was successful.
    ///
    /// `Udf` automatically closes the file system when dropped, so this function need only be used
    /// if the caller wishes to differentiate between success and failure.
    pub fn close(mut self) -> bool {
        if self.handle.is_null() {
            true
        } else {
            let ret = unsafe { udf_close(self.handle) };
            self.handle = null_mut();
            ret
        }
    }

    /// Obtains a handle to the root directory of the UDF file system.
    ///
    /// If `partition` is `Some(_)`, obtains a handle to the root directory of the partition with
    /// that number. If `partition` is `None`, obtains a handle to the first root directory that
    /// can be found.
    ///
    /// Returns `None` if the root directory could not be obtained.
    pub fn get_root<'u>(&'u self, partition: Option<u16>) -> Option<UdfDirEntry<'u>> {
        let (any_partition, partition_number) = match partition {
            Some(p) => (false, p),
            None => (true, 0),
        };
        let root_handle = unsafe {
            udf_get_root(self.handle, any_partition, partition_number)
        };
        if root_handle.is_null() {
            None
        } else {
            Some(UdfDirEntry {
                handle: root_handle,
                udf_ref: self,
            })
        }
    }

    /// Obtains a handle to the directory entry with the given path on the given UDF partition.
    ///
    /// The `partition` argument works just as with [`get_root`](Self::get_root).
    ///
    /// Valid delimiters within `path` are `/` (0x2F) and `\` (0x5C).
    ///
    /// Returns `None` if the entry could not be obtained, e.g. because it does not exist.
    pub fn get_by_path<'u>(&'u self, partition: Option<u16>, path: &CStr) -> Option<UdfDirEntry<'u>> {
        let root = self.get_root(partition)?;
        let dir_entry_handle = unsafe {
            udf_fopen(root.handle, path.as_ptr())
        };
        if dir_entry_handle.is_null() {
            None
        } else {
            Some(UdfDirEntry {
                handle: dir_entry_handle,
                udf_ref: self,
            })
        }
    }
}
impl Drop for Udf {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { udf_close(self.handle) };
            self.handle = null_mut();
        }
    }
}

/// An entry in a UDF directory, as well as a quasi-iterator to reach its successive siblings.
///
/// Most associated functions operate on the current entry; [`advance`](Self::advance) consumes the
/// current entry and returns the next sibling (or `None` if there are no more siblings).
pub struct UdfDirEntry<'u> {
    handle: *mut udf_dirent_t,
    udf_ref: &'u Udf,
}
impl<'u> UdfDirEntry<'u> {
    /// Returns the name of this directory entry.
    pub fn name(&self) -> Option<&CStr> {
        let name_ptr = unsafe { udf_get_filename(self.handle) };
        if name_ptr.is_null() {
            None
        } else {
            let c_str = unsafe { CStr::from_ptr(name_ptr) };
            Some(c_str)
        }
    }

    /// Returns whether this directory entry is itself a directory.
    pub fn is_dir(&self) -> bool {
        unsafe { udf_is_dir(self.handle) }
    }

    /// Returns the length of the current file, or `None` if an error occurs.
    ///
    /// An error may occur e.g. if the current entry is a directory and not a file.
    pub fn file_length(&self) -> Option<u64> {
        let length = unsafe { udf_get_file_length(self.handle) };
        if length == 2147483647 {
            None
        } else {
            Some(length)
        }
    }

    /// Reads data from the current position in the current file.
    ///
    /// `buf.len()` should be a multiple of [`BLOCK_LENGTH`].
    ///
    /// Returns the number of bytes read, or a negative value in case of failure.
    pub fn read(&mut self, buf: &mut [u8]) -> isize {
        let blocks_to_read = buf.len() / BLOCK_LENGTH;
        if blocks_to_read == 0 {
            return 0;
        }

        unsafe {
            udf_read_block(
                self.handle,
                buf.as_mut_ptr() as *mut c_void,
                blocks_to_read,
            )
        }
    }

    /// Returns the `UdfDirEntry` representing the first entry within this entry, which must be a
    /// directory.
    ///
    /// Returns `None` if this fails, e.g. because the current entry is not a directory.
    pub fn descend(&self) -> Option<Self> {
        let child_handle = unsafe {
            udf_opendir(self.handle)
        };
        if child_handle.is_null() {
            None
        } else {
            Some(Self {
                handle: child_handle,
                udf_ref: self.udf_ref,
            })
        }
    }

    /// Returns the `UdfDirEntry` representing the next entry in the same directory, if any.
    ///
    /// Consumes the current entry. Returns `None` if there are no more entries.
    pub fn advance(mut self) -> Option<Self> {
        let next_handle = unsafe { udf_readdir(self.handle) };
        self.handle = next_handle;
        if next_handle.is_null() {
            None
        } else {
            Some(self)
        }
    }

    /// Repeatedly advances the current entry, stopping at the one that fulfills a predicate.
    ///
    /// The predicate is tested against the current entry and all subsequent entries, but not
    /// against any previous entry.
    ///
    /// Returns `Some(_)` referencing the first entry encountered that fulfills the predicate.
    /// (Returns `Some(self)` if the current entry fulfills the predicate.) Returns `None` if none
    /// of the encountered entries fulfilled the predicate.
    pub fn advance_until<P: FnMut(&Self) -> bool>(self, mut predicate: P) -> Option<Self> {
        let mut walker = self;
        loop {
            if predicate(&walker) {
                return Some(walker);
            }

            let next_stop_opt = walker.advance();
            if let Some(next_stop) = next_stop_opt {
                walker = next_stop;
            } else {
                return None;
            }
        }
    }
}
impl<'u> Drop for UdfDirEntry<'u> {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { udf_dirent_free(self.handle) };
            self.handle = null_mut();
        }
    }
}
