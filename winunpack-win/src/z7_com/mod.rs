//! 7-Zip COM API.


#![allow(non_camel_case_types, non_snake_case)]


pub mod archive;
pub mod coder;
pub mod folder;
pub mod folder_archive;
pub mod progress;
pub mod stream;


pub type PROPID = u32;
pub type VARTYPE = u16;
pub type wchar_t = u16;


#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FILETIME {
    pub dwLowDateTime: u32,
    pub dwHighDateTime: u32,
}


pub fn to_wide_nul_terminated_string(s: &str) -> Vec<wchar_t> {
    assert!(!s.contains('\u{00}'));
    let mut words: Vec<wchar_t> = s.encode_utf16().collect();
    words.push(0x0000);
    words
}
