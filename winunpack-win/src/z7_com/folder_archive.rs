#![allow(non_camel_case_types, non_snake_case)]


use windows::Win32::Foundation::FILETIME;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows_core::{interface, IUnknown, IUnknown_Vtbl, HRESULT};

use crate::z7_com::progress::{IProgress, IProgress_Impl, IProgress_Vtbl};
use crate::z7_com::stream::ISequentialOutStream;
use crate::z7_com::PROPID;


#[interface("23170F69-40C1-278A-0000-000100070000")]
pub unsafe trait IFolderArchiveExtractCallback : IProgress {
    fn AskOverwrite(
        &self,
        exist_name: *const u16, exist_time: *const FILETIME, exist_size: *const u64,
        new_name: *const u16, new_time: *const FILETIME, new_size: *const u64,
        answer: *mut i32,
    ) -> HRESULT;
    fn PrepareOperation(&self, name: *const u16, is_folder: i32, ask_extract_mode: i32, position: *const u64) -> HRESULT;
    fn MessageError(&self, message: *const u16) -> HRESULT;
    fn SetOperationResult(&self, op_res: i32, encrypted: i32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000100080000")]
pub unsafe trait IFolderArchiveExtractCallback2 : IUnknown {
    fn ReportExtractResult(&self, op_res: i32, encrypted: i32, name: *mut u16) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-0001000B0000")]
pub unsafe trait IFolderArchiveUpdateCallback : IProgress {
    fn CompressOperation(&self, name: *mut u16) -> HRESULT;
    fn DeleteOperation(&self, name: *mut u16) -> HRESULT;
    fn OperationResult(&self, op_res: i32) -> HRESULT;
    fn UpdateErrorMessage(&self, message: *mut u16) -> HRESULT;
    fn SetNumFiles(&self, num_files: u64) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-0001000F0000")]
pub unsafe trait IOutFolderArchive : IUnknown {
    fn SetFolder(&self, folder: *mut IFolderFolder) -> HRESULT;
    fn SetFiles(&self, folder_prefix: *const u16, names: *const *const u16, num_names: u32) -> HRESULT;
    fn DeleteItems(
        &self,
        out_archive_stream: *mut ISequentialOutStream,
        indices: *const u32, num_items: u32, update_callback: *mut IFolderArchiveUpdateCallback,
    ) -> HRESULT;
    fn DoOperation(
        &self,
        requested_paths: *mut FStringVector,
        processed_paths: *mut FStringVector,
        codecs: *mut CCodecs, index: i32,
        out_archive_stream: *mut ISequentialOutStream, state_actions: *const u8, sfx_module: *const u16,
        update_callback: *mut IFolderUpdateCallback,
    ) -> HRESULT;
    fn DoOperation2(
        &self,
        requested_paths: *mut FStringVector,
        processed_paths: *mut FStringVector,
        out_archive_stream: *mut ISequentialOutStream, state_actions: *const u8, sfx_module: *const u16,
        update_callback: *mut IFolderUpdateCallback,
    ) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000100100000")]
pub unsafe trait IFolderArchiveUpdateCallback2 : IUnknown {
    fn OpenFileError(&self, path: *mut u16, error_code: HRESULT) -> HRESULT;
    fn ReadingFileError(&self, path: *mut u16, error_code: HRESULT) -> HRESULT;
    fn ReportExtractResult(&self, op_res: i32, is_encrypted: i32, path: *const u16) -> HRESULT;
    fn ReportUpdateOperation(&self, notify_op: i32, path: *const u16, is_dir: i32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000100110000")]
pub unsafe trait IFolderScanProgress : IUnknown {
    fn ScanError(&self, path: *const u16, error_code: HRESULT) -> HRESULT;
    fn ScanProgress(&self, num_folders: u64, num_files: u64, total_size: u64, path: *const u16, is_dir: i32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000100120000")]
pub unsafe trait IFolderSetZoneIdMode : IUnknown {
    fn SetZoneIdMode(&self, zone_mode: NExtract::NZoneIdMode::EEnum) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000100130000")]
pub unsafe trait IFolderSetZoneIdFile : IUnknown {
    fn SetZoneIdFile(&self, data: *const u8, size: u32) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000100140000")]
pub unsafe trait IFolderArchiveUpdateCallback_MoveArc : IUnknown {
    fn MoveArc_Start(&self, src_temp_path: *const u16, dest_final_path: *const u16, size: u64, update_mode: i32) -> HRESULT;
    fn MoveArc_Progress(&self, total_size: u64, current_size: u64) -> HRESULT;
    fn MoveArc_Finish(&self) -> HRESULT;
    fn Before_ArcReopen(&self) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000100200000")]
pub unsafe trait IGetProp : IUnknown {
    fn GetProp(&self, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
}

#[interface("23170F69-40C1-278A-0000-000100310000")]
pub unsafe trait IFolderExtractToStreamCallback : IUnknown {
    fn UseExtractToStream(&self, res: *mut i32) -> HRESULT;
    fn GetStream7(&self, name: *const u16, is_dir: i32, out_stream: *mut *mut ISequentialOutStream, ask_extract_mode: i32, get_prop: *mut IGetProp) -> HRESULT;
    fn PrepareOperation7(&self, ask_extract_mode: i32) -> HRESULT;
    fn SetOperationResult8(&self, result_e_operation_result: i32, encrypted: i32, size: u64) -> HRESULT;
}
