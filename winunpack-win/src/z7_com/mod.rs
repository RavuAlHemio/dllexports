//! 7-Zip COM API.


#![allow(non_camel_case_types, non_snake_case)]


pub mod coder;
pub mod folder_archive;
pub mod progress;
pub mod stream;


pub type PROPID = u32;


#[interface("23170F69-40C1-278A-0000-000600600000")]
unsafe trait IInArchive : IUnknown {
    fn Open(&self, stream: IInStream, max_check_start_position: *mut u64, open_callback: IArchiveOpenCallback) -> HRESULT;
    fn Close(&self);
    fn GetNumberOfItems(&self) -> u32;
}
