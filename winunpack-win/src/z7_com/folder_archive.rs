use std::ffi::c_void;

use from_to_repr::from_to_other;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows_core::{HRESULT, IUnknown, IUnknown_Vtbl, OutRef};
use winunpack_macros::interface_7zip;

use crate::z7_com::{FILETIME, PROPID, wchar_t};
use crate::z7_com::folder::IFolderFolder;
use crate::z7_com::progress::{IProgress, IProgress_Impl, IProgress_Vtbl};
use crate::z7_com::stream::ISequentialOutStream;


// implement later (or not)
pub type FStringVector = c_void;
pub type CCodecs = c_void;


#[interface_7zip(1, 0x07)]
pub unsafe trait IFolderArchiveExtractCallback : IProgress {
    fn AskOverwrite(
        &self,
        exist_name: *const wchar_t, exist_time: *const FILETIME, exist_size: *const u64,
        new_name: *const wchar_t, new_time: *const FILETIME, new_size: *const u64,
        answer: *mut i32,
    ) -> HRESULT;
    fn PrepareOperation(&self, name: *const wchar_t, is_folder: i32, ask_extract_mode: i32, position: *const u64) -> HRESULT;
    fn MessageError(&self, message: *const wchar_t) -> HRESULT;
    fn SetOperationResult(&self, op_res: i32, encrypted: i32) -> HRESULT;
}

#[interface_7zip(1, 0x08)]
pub unsafe trait IFolderArchiveExtractCallback2 : IUnknown {
    fn ReportExtractResult(&self, op_res: i32, encrypted: i32, name: *mut wchar_t) -> HRESULT;
}

#[interface_7zip(1, 0x0B)]
pub unsafe trait IFolderArchiveUpdateCallback : IProgress {
    fn CompressOperation(&self, name: *mut wchar_t) -> HRESULT;
    fn DeleteOperation(&self, name: *mut wchar_t) -> HRESULT;
    fn OperationResult(&self, op_res: i32) -> HRESULT;
    fn UpdateErrorMessage(&self, message: *mut wchar_t) -> HRESULT;
    fn SetNumFiles(&self, num_files: u64) -> HRESULT;
}

#[interface_7zip(1, 0x0F)]
pub unsafe trait IOutFolderArchive : IUnknown {
    fn SetFolder(&self, folder: IFolderFolder) -> HRESULT;
    fn SetFiles(&self, folder_prefix: *const wchar_t, names: *const *const wchar_t, num_names: u32) -> HRESULT;
    fn DeleteItems(
        &self,
        out_archive_stream: ISequentialOutStream,
        indices: *const u32, num_items: u32, update_callback: IFolderArchiveUpdateCallback,
    ) -> HRESULT;
    fn DoOperation(
        &self,
        requested_paths: *mut FStringVector,
        processed_paths: *mut FStringVector,
        codecs: *mut CCodecs, index: i32,
        out_archive_stream: ISequentialOutStream, state_actions: *const u8, sfx_module: *const wchar_t,
        update_callback: IFolderArchiveUpdateCallback,
    ) -> HRESULT;
    fn DoOperation2(
        &self,
        requested_paths: *mut FStringVector,
        processed_paths: *mut FStringVector,
        out_archive_stream: ISequentialOutStream, state_actions: *const u8, sfx_module: *const wchar_t,
        update_callback: IFolderArchiveUpdateCallback,
    ) -> HRESULT;
}

#[interface_7zip(1, 0x10)]
pub unsafe trait IFolderArchiveUpdateCallback2 : IUnknown {
    fn OpenFileError(&self, path: *mut wchar_t, error_code: HRESULT) -> HRESULT;
    fn ReadingFileError(&self, path: *mut wchar_t, error_code: HRESULT) -> HRESULT;
    fn ReportExtractResult(&self, op_res: i32, is_encrypted: i32, path: *const wchar_t) -> HRESULT;
    fn ReportUpdateOperation(&self, notify_op: i32, path: *const wchar_t, is_dir: i32) -> HRESULT;
}

#[interface_7zip(1, 0x11)]
pub unsafe trait IFolderScanProgress : IUnknown {
    fn ScanError(&self, path: *const wchar_t, error_code: HRESULT) -> HRESULT;
    fn ScanProgress(&self, num_folders: u64, num_files: u64, total_size: u64, path: *const wchar_t, is_dir: i32) -> HRESULT;
}

#[derive(Clone, Copy, Debug)]
#[from_to_other(base_type = i32, derive_compare = "as_int")]
pub enum NZoneIdMode {
    None = 0,
    All = 1,
    Office = 2,
    Other(i32),
}

#[interface_7zip(1, 0x12)]
pub unsafe trait IFolderSetZoneIdMode : IUnknown {
    fn SetZoneIdMode(&self, zone_mode: i32 /* NZoneIdMode */) -> HRESULT;
}

#[interface_7zip(1, 0x13)]
pub unsafe trait IFolderSetZoneIdFile : IUnknown {
    fn SetZoneIdFile(&self, data: *const u8, size: u32) -> HRESULT;
}

#[interface_7zip(1, 0x14)]
pub unsafe trait IFolderArchiveUpdateCallback_MoveArc : IUnknown {
    fn MoveArc_Start(&self, src_temp_path: *const wchar_t, dest_final_path: *const wchar_t, size: u64, update_mode: i32) -> HRESULT;
    fn MoveArc_Progress(&self, total_size: u64, current_size: u64) -> HRESULT;
    fn MoveArc_Finish(&self) -> HRESULT;
    fn Before_ArcReopen(&self) -> HRESULT;
}

#[interface_7zip(1, 0x20)]
pub unsafe trait IGetProp : IUnknown {
    fn GetProp(&self, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
}

#[interface_7zip(1, 0x31)]
pub unsafe trait IFolderExtractToStreamCallback : IUnknown {
    fn UseExtractToStream(&self, res: *mut i32) -> HRESULT;
    fn GetStream7(&self, name: *const wchar_t, is_dir: i32, out_stream: OutRef<ISequentialOutStream>, ask_extract_mode: i32, get_prop: IGetProp) -> HRESULT;
    fn PrepareOperation7(&self, ask_extract_mode: i32) -> HRESULT;
    fn SetOperationResult8(&self, result_e_operation_result: i32, encrypted: i32, size: u64) -> HRESULT;
}
