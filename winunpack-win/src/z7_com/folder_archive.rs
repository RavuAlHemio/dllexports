use std::ffi::c_void;
use std::ptr::{null, null_mut};

use from_to_repr::{from_to_other, FromToRepr};
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows_core::{Error, HRESULT, IUnknown, IUnknown_Vtbl, Type};
use winunpack_macros::interface_7zip;

use crate::z7_com::{FILETIME, PROPID, to_wide_nul_terminated_string, wchar_t};
use crate::z7_com::folder::IFolderFolder;
use crate::z7_com::progress::{IProgress, IProgress_Impl, IProgress_Vtbl};
use crate::z7_com::stream::ISequentialOutStream;


// implement later (or not)
pub type FStringVector = c_void;
pub type CCodecs = c_void;


#[interface_7zip(1, 0x07)]
pub unsafe trait IFolderArchiveExtractCallback : IProgress {
    fn AskOverwrite_Raw(
        &self,
        exist_name: *const wchar_t, exist_time: *const FILETIME, exist_size: *const u64,
        new_name: *const wchar_t, new_time: *const FILETIME, new_size: *const u64,
        answer: *mut i32,
    ) -> HRESULT;
    fn PrepareOperation_Raw(&self, name: *const wchar_t, is_folder: i32, ask_extract_mode: i32, position: *const u64) -> HRESULT;
    fn MessageError_Raw(&self, message: *const wchar_t) -> HRESULT;
    fn SetOperationResult_Raw(&self, op_res: i32, encrypted: i32) -> HRESULT;
}

pub trait IFolderArchiveExtractCallback_Ext {
    fn AskOverwrite(
        &self,
        exist_name: &str, exist_time: FILETIME, exist_size: u64,
        new_name: &str, new_time: FILETIME, new_size: u64,
    ) -> Result<i32, Error>;
    fn PrepareOperation(&self, name: &str, is_folder: i32, ask_extract_mode: i32, position: u64) -> Result<(), Error>;
    fn MessageError(&self, message: &str) -> Result<(), Error>;
    fn SetOperationResult(&self, op_res: i32, encrypted: i32) -> Result<(), Error>;
}
impl<T: IFolderArchiveExtractCallback_Impl> IFolderArchiveExtractCallback_Ext for T {
    fn AskOverwrite(
        &self,
        exist_name: &str, exist_time: FILETIME, exist_size: u64,
        new_name: &str, new_time: FILETIME, new_size: u64,
    ) -> Result<i32, Error> {
        let exist_name_w = to_wide_nul_terminated_string(&exist_name);
        let new_name_w = to_wide_nul_terminated_string(&new_name);

        let mut answer = 0;
        unsafe {
            self.AskOverwrite_Raw(
                exist_name_w.as_ptr(), &exist_time, &exist_size,
                new_name_w.as_ptr(), &new_time, &new_size,
                &mut answer,
            )
                .map(|| answer)
        }
    }

    fn PrepareOperation(&self, name: &str, is_folder: i32, ask_extract_mode: i32, position: u64) -> Result<(), Error> {
        let name_w: Vec<wchar_t> = to_wide_nul_terminated_string(name);

        unsafe {
            self.PrepareOperation_Raw(name_w.as_ptr(), is_folder, ask_extract_mode, &position)
                .ok()
        }
    }

    fn MessageError(&self, message: &str) -> Result<(), Error> {
        let message_w: Vec<wchar_t> = to_wide_nul_terminated_string(message);

        unsafe {
            self.MessageError_Raw(message_w.as_ptr())
                .ok()
        }
    }

    fn SetOperationResult(&self, op_res: i32, encrypted: i32) -> Result<(), Error> {
        unsafe {
            self.SetOperationResult_Raw(op_res, encrypted)
                .ok()
        }
    }
}

#[interface_7zip(1, 0x08)]
pub unsafe trait IFolderArchiveExtractCallback2 : IUnknown {
    fn ReportExtractResult_Raw(&self, op_res: i32, encrypted: i32, name: *const wchar_t) -> HRESULT;
}

pub trait IFolderArchiveExtractCallback2_Ext {
    fn ReportExtractResult(&self, op_res: i32, encrypted: i32, name: &str) -> Result<(), Error>;
}
impl<T: IFolderArchiveExtractCallback2_Impl> IFolderArchiveExtractCallback2_Ext for T {
    fn ReportExtractResult(&self, op_res: i32, encrypted: i32, name: &str) -> Result<(), Error> {
        let name_w = to_wide_nul_terminated_string(name);

        unsafe {
            self.ReportExtractResult_Raw(op_res, encrypted, name_w.as_ptr())
                .ok()
        }
    }
}

#[interface_7zip(1, 0x0B)]
pub unsafe trait IFolderArchiveUpdateCallback : IProgress {
    fn CompressOperation_Raw(&self, name: *const wchar_t) -> HRESULT;
    fn DeleteOperation_Raw(&self, name: *const wchar_t) -> HRESULT;
    fn OperationResult_Raw(&self, op_res: i32) -> HRESULT;
    fn UpdateErrorMessage_Raw(&self, message: *const wchar_t) -> HRESULT;
    fn SetNumFiles_Raw(&self, num_files: u64) -> HRESULT;
}

pub trait IFolderArchiveUpdateCallback_Ext {
    fn CompressOperation(&self, name: &str) -> Result<(), Error>;
    fn DeleteOperation(&self, name: &str) -> Result<(), Error>;
    fn OperationResult(&self, op_res: i32) -> Result<(), Error>;
    fn UpdateErrorMessage(&self, message: &str) -> Result<(), Error>;
    fn SetNumFiles(&self, num_files: u64) -> Result<(), Error>;
}
impl<T: IFolderArchiveUpdateCallback_Impl> IFolderArchiveUpdateCallback_Ext for T {
    fn CompressOperation(&self, name: &str) -> Result<(), Error> {
        let name_w = to_wide_nul_terminated_string(name);

        unsafe {
            self.CompressOperation_Raw(name_w.as_ptr())
                .ok()
        }
    }

    fn DeleteOperation(&self, name: &str) -> Result<(), Error> {
        let name_w = to_wide_nul_terminated_string(name);

        unsafe {
            self.DeleteOperation_Raw(name_w.as_ptr())
                .ok()
        }
    }

    fn OperationResult(&self, op_res: i32) -> Result<(), Error> {
        unsafe {
            self.OperationResult_Raw(op_res)
                .ok()
        }
    }

    fn UpdateErrorMessage(&self, message: &str) -> Result<(), Error> {
        let message_w = to_wide_nul_terminated_string(message);

        unsafe {
            self.UpdateErrorMessage_Raw(message_w.as_ptr())
                .ok()
        }
    }

    fn SetNumFiles(&self, num_files: u64) -> Result<(), Error> {
        unsafe {
            self.SetNumFiles_Raw(num_files)
                .ok()
        }
    }
}

#[interface_7zip(1, 0x0F)]
pub unsafe trait IOutFolderArchive : IUnknown {
    fn SetFolder_Raw(&self, folder: IFolderFolder) -> HRESULT;
    fn SetFiles_Raw(&self, folder_prefix: *const wchar_t, names: *const *const wchar_t, num_names: u32) -> HRESULT;
    fn DeleteItems_Raw(
        &self,
        out_archive_stream: ISequentialOutStream,
        indices: *const u32, num_items: u32, update_callback: IFolderArchiveUpdateCallback,
    ) -> HRESULT;
    fn DoOperation_Raw(
        &self,
        requested_paths: *mut FStringVector,
        processed_paths: *mut FStringVector,
        codecs: *mut CCodecs, index: i32,
        out_archive_stream: ISequentialOutStream, state_actions: *const u8, sfx_module: *const wchar_t,
        update_callback: IFolderArchiveUpdateCallback,
    ) -> HRESULT;
    fn DoOperation2_Raw(
        &self,
        requested_paths: *mut FStringVector,
        processed_paths: *mut FStringVector,
        out_archive_stream: ISequentialOutStream, state_actions: *const u8, sfx_module: *const wchar_t,
        update_callback: IFolderArchiveUpdateCallback,
    ) -> HRESULT;
}

#[derive(Clone, Copy, Debug, Eq, FromToRepr, Hash, PartialEq, PartialOrd)]
#[repr(usize)]
pub enum PairState {
    NotMasked = 0,
    OnlyInArchive = 1,
    OnlyOnDisk = 2,
    NewInArchive = 3,
    OldInArchive = 4,
    SameFiles = 5,
    UnknownNewerFiles = 6,
}
const PAIR_STATE_VALUES: usize = 7;

#[derive(Clone, Copy, Debug, Eq, FromToRepr, Hash, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum PairAction {
    Ignore = 0,
    Copy = 1,
    Compress = 2,
    CompressAsAnti = 3,
}

pub trait IOutFolderArchive_Ext {
    fn SetFolder(&self, folder: IFolderFolder) -> Result<(), Error>;
    fn SetFiles(&self, folder_prefix: &str, names: &[&str]) -> Result<(), Error>;
    fn DeleteItems(
        &self,
        out_archive_stream: ISequentialOutStream,
        indices: &[u32], update_callback: IFolderArchiveUpdateCallback,
    ) -> Result<(), Error>;
    // don't offer DoOperation because we haven't defined CCodecs
    fn DoOperation2(
        &self,
        // don't offer requested_paths and processed_paths because we haven't define FStringVector
        out_archive_stream: ISequentialOutStream, state_actions: &[PairAction], sfx_module: Option<&str>,
        update_callback: IFolderArchiveUpdateCallback,
    ) -> Result<(), Error>;
}
impl<T: IOutFolderArchive_Impl> IOutFolderArchive_Ext for T {
    fn SetFolder(&self, folder: IFolderFolder) -> Result<(), Error> {
        unsafe {
            self.SetFolder_Raw(folder)
                .ok()
        }
    }

    fn SetFiles(&self, folder_prefix: &str, names: &[&str]) -> Result<(), Error> {
        let folder_prefix_w = to_wide_nul_terminated_string(&folder_prefix);

        let mut names_w = Vec::with_capacity(names.len());
        for name in names {
            let name_w = to_wide_nul_terminated_string(name);
            names_w.push(name_w);
        }

        let name_ptrs: Vec<*const u16> = names_w.iter()
            .map(|n| n.as_ptr())
            .collect();

        let num_names: u32 = name_ptrs.len().try_into().unwrap();

        unsafe {
            self.SetFiles_Raw(folder_prefix_w.as_ptr(), name_ptrs.as_ptr(), num_names)
                .ok()
        }
    }

    fn DeleteItems(
        &self,
        out_archive_stream: ISequentialOutStream,
        indices: &[u32], update_callback: IFolderArchiveUpdateCallback,
    ) -> Result<(), Error> {
        let num_items = indices.len().try_into().unwrap();
        unsafe {
            self.DeleteItems_Raw(out_archive_stream, indices.as_ptr(), num_items, update_callback)
                .ok()
        }
    }

    fn DoOperation2(
        &self,
        out_archive_stream: ISequentialOutStream, state_actions: &[PairAction], sfx_module: Option<&str>,
        update_callback: IFolderArchiveUpdateCallback,
    ) -> Result<(), Error> {
        assert_eq!(state_actions.len(), PAIR_STATE_VALUES);

        let sfx_module_w: Vec<wchar_t>;
        let sfx_module_ptr = if let Some(sm) = sfx_module {
            sfx_module_w = to_wide_nul_terminated_string(sm);
            sfx_module_w.as_ptr()
        } else {
            null()
        };

        let mut state_actions_array = [0; PAIR_STATE_VALUES];
        for (action_val, action) in state_actions_array.iter_mut().zip(state_actions.iter()) {
            *action_val = action.into_repr();
        }

        unsafe {
            self.DoOperation2_Raw(
                null_mut(),
                null_mut(),
                out_archive_stream,
                state_actions_array.as_ptr(),
                sfx_module_ptr,
                update_callback,
            )
                .ok()
        }
    }
}

#[interface_7zip(1, 0x10)]
pub unsafe trait IFolderArchiveUpdateCallback2 : IUnknown {
    fn OpenFileError_Raw(&self, path: *const wchar_t, error_code: HRESULT) -> HRESULT;
    fn ReadingFileError_Raw(&self, path: *const wchar_t, error_code: HRESULT) -> HRESULT;
    fn ReportExtractResult_Raw(&self, op_res: i32, is_encrypted: i32, path: *const wchar_t) -> HRESULT;
    fn ReportUpdateOperation_Raw(&self, notify_op: i32, path: *const wchar_t, is_dir: i32) -> HRESULT;
}

pub trait IFolderArchiveUpdateCallback2_Ext {
    fn OpenFileError(&self, path: &str, error_code: HRESULT) -> Result<(), Error>;
    fn ReadingFileError(&self, path: &str, error_code: HRESULT) -> Result<(), Error>;
    fn ReportExtractResult(&self, op_res: i32, is_encrypted: i32, path: &str) -> Result<(), Error>;
    fn ReportUpdateOperation(&self, notify_op: i32, path: &str, is_dir: i32) -> Result<(), Error>;
}
impl<T: IFolderArchiveUpdateCallback2_Impl> IFolderArchiveUpdateCallback2_Ext for T {
    fn OpenFileError(&self, path: &str, error_code: HRESULT) -> Result<(), Error> {
        let path_w = to_wide_nul_terminated_string(path);

        unsafe {
            self.OpenFileError_Raw(path_w.as_ptr(), error_code)
                .ok()
        }
    }

    fn ReadingFileError(&self, path: &str, error_code: HRESULT) -> Result<(), Error> {
        let path_w = to_wide_nul_terminated_string(path);

        unsafe {
            self.ReadingFileError_Raw(path_w.as_ptr(), error_code)
                .ok()
        }
    }

    fn ReportExtractResult(&self, op_res: i32, is_encrypted: i32, path: &str) -> Result<(), Error> {
        let path_w = to_wide_nul_terminated_string(path);

        unsafe {
            self.ReportExtractResult_Raw(op_res, is_encrypted, path_w.as_ptr())
                .ok()
        }
    }

    fn ReportUpdateOperation(&self, notify_op: i32, path: &str, is_dir: i32) -> Result<(), Error> {
        let path_w = to_wide_nul_terminated_string(path);

        unsafe {
            self.ReportUpdateOperation_Raw(notify_op, path_w.as_ptr(), is_dir)
                .ok()
        }
    }
}

#[interface_7zip(1, 0x11)]
pub unsafe trait IFolderScanProgress : IUnknown {
    fn ScanError_Raw(&self, path: *const wchar_t, error_code: HRESULT) -> HRESULT;
    fn ScanProgress_Raw(&self, num_folders: u64, num_files: u64, total_size: u64, path: *const wchar_t, is_dir: i32) -> HRESULT;
}

pub trait IFolderScanProgress_Ext {
    fn ScanError(&self, path: &str, error_code: HRESULT) -> Result<(), Error>;
    fn ScanProgress(&self, num_folders: u64, num_files: u64, total_size: u64, path: &str, is_dir: i32) -> Result<(), Error>;
}
impl<T: IFolderScanProgress_Impl> IFolderScanProgress_Ext for T {
    fn ScanError(&self, path: &str, error_code: HRESULT) -> Result<(), Error> {
        let path_w = to_wide_nul_terminated_string(path);

        unsafe {
            self.ScanError_Raw(path_w.as_ptr(), error_code)
                .ok()
        }
    }

    fn ScanProgress(&self, num_folders: u64, num_files: u64, total_size: u64, path: &str, is_dir: i32) -> Result<(), Error> {
        let path_w = to_wide_nul_terminated_string(path);

        unsafe {
            self.ScanProgress_Raw(num_folders, num_files, total_size, path_w.as_ptr(), is_dir)
                .ok()
        }
    }
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
    fn SetZoneIdMode_Raw(&self, zone_mode: i32 /* NZoneIdMode */) -> HRESULT;
}

pub trait IFolderSetZoneIdMode_Ext {
    fn SetZoneIdMode(&self, zone_mode: NZoneIdMode) -> Result<(), Error>;
}
impl<T: IFolderSetZoneIdMode_Impl> IFolderSetZoneIdMode_Ext for T {
    fn SetZoneIdMode(&self, zone_mode: NZoneIdMode) -> Result<(), Error> {
        let zone_mode_i32 = zone_mode.to_base_type();
        unsafe {
            self.SetZoneIdMode_Raw(zone_mode_i32)
                .ok()
        }
    }
}

#[interface_7zip(1, 0x13)]
pub unsafe trait IFolderSetZoneIdFile : IUnknown {
    fn SetZoneIdFile_Raw(&self, data: *const u8, size: u32) -> HRESULT;
}

pub trait IFolderSetZoneIdFile_Ext {
    fn SetZoneIdMode(&self, data: &[u8]) -> Result<(), Error>;
}
impl<T: IFolderSetZoneIdFile_Impl> IFolderSetZoneIdFile_Ext for T {
    fn SetZoneIdMode(&self, data: &[u8]) -> Result<(), Error> {
        let size: u32 = data.len().try_into().unwrap();
        unsafe {
            self.SetZoneIdFile_Raw(data.as_ptr(), size)
                .ok()
        }
    }
}

#[interface_7zip(1, 0x14)]
pub unsafe trait IFolderArchiveUpdateCallback_MoveArc : IUnknown {
    fn MoveArc_Start_Raw(&self, src_temp_path: *const wchar_t, dest_final_path: *const wchar_t, size: u64, update_mode: i32) -> HRESULT;
    fn MoveArc_Progress_Raw(&self, total_size: u64, current_size: u64) -> HRESULT;
    fn MoveArc_Finish_Raw(&self) -> HRESULT;
    fn Before_ArcReopen_Raw(&self) -> HRESULT;
}

pub trait IFolderArchiveUpdateCallback_MoveArc_Ext {
    fn MoveArc_Start(&self, src_temp_path: &str, dest_final_path: &str, size: u64, update_mode: i32) -> Result<(), Error>;
    fn MoveArc_Progress(&self, total_size: u64, current_size: u64) -> Result<(), Error>;
    fn MoveArc_Finish(&self) -> Result<(), Error>;
    fn Before_ArcReopen(&self) -> Result<(), Error>;
}
impl<T: IFolderArchiveUpdateCallback_MoveArc_Impl> IFolderArchiveUpdateCallback_MoveArc_Ext for T {
    fn MoveArc_Start(&self, src_temp_path: &str, dest_final_path: &str, size: u64, update_mode: i32) -> Result<(), Error> {
        let src_temp_path_w = to_wide_nul_terminated_string(src_temp_path);
        let dest_final_path_w = to_wide_nul_terminated_string(dest_final_path);

        unsafe {
            self.MoveArc_Start_Raw(src_temp_path_w.as_ptr(), dest_final_path_w.as_ptr(), size, update_mode)
                .ok()
        }
    }

    fn MoveArc_Progress(&self, total_size: u64, current_size: u64) -> Result<(), Error> {
        unsafe {
            self.MoveArc_Progress_Raw(total_size, current_size)
                .ok()
        }
    }

    fn MoveArc_Finish(&self) -> Result<(), Error> {
        unsafe {
            self.MoveArc_Finish_Raw()
                .ok()
        }
    }

    fn Before_ArcReopen(&self) -> Result<(), Error> {
        unsafe {
            self.Before_ArcReopen_Raw()
                .ok()
        }
    }
}

#[interface_7zip(1, 0x20)]
pub unsafe trait IGetProp : IUnknown {
    fn GetProp_Raw(&self, prop_id: PROPID, value: *mut PROPVARIANT) -> HRESULT;
}

pub trait IGetProp_Ext {
    fn GetProp(&self, prop_id: PROPID) -> Result<PROPVARIANT, Error>;
}
impl<T: IGetProp_Impl> IGetProp_Ext for T {
    fn GetProp(&self, prop_id: PROPID) -> Result<PROPVARIANT, Error> {
        let mut value = PROPVARIANT::default();
        unsafe {
            self.GetProp_Raw(prop_id, &mut value)
                .map(|| value)
        }
    }
}

#[interface_7zip(1, 0x31)]
pub unsafe trait IFolderExtractToStreamCallback : IUnknown {
    fn UseExtractToStream_Raw(&self, res: *mut i32) -> HRESULT;
    fn GetStream7_Raw(&self, name: *const wchar_t, is_dir: i32, out_stream: *mut *mut c_void, ask_extract_mode: i32, get_prop: IGetProp) -> HRESULT;
    fn PrepareOperation7_Raw(&self, ask_extract_mode: i32) -> HRESULT;
    fn SetOperationResult8_Raw(&self, result_e_operation_result: i32, encrypted: i32, size: u64) -> HRESULT;
}

pub trait IFolderExtractToStreamCallback_Ext {
    fn UseExtractToStream(&self) -> Result<i32, Error>;
    fn GetStream7(&self, name: &str, is_dir: i32, ask_extract_mode: i32, get_prop: IGetProp) -> Result<ISequentialOutStream, Error>;
    fn PrepareOperation7(&self, ask_extract_mode: i32) -> Result<(), Error>;
    fn SetOperationResult8(&self, result_e_operation_result: i32, encrypted: i32, size: u64) -> Result<(), Error>;
}
impl<T: IFolderExtractToStreamCallback_Impl> IFolderExtractToStreamCallback_Ext for T {
    fn UseExtractToStream(&self) -> Result<i32, Error> {
        let mut res = 0;
        unsafe {
            self.UseExtractToStream_Raw(&mut res)
                .map(|| res)
        }
    }

    fn GetStream7(&self, name: &str, is_dir: i32, ask_extract_mode: i32, get_prop: IGetProp) -> Result<ISequentialOutStream, Error> {
        let name_w = to_wide_nul_terminated_string(name);

        let mut out_stream_raw = std::ptr::null_mut();
        unsafe {
            self.GetStream7_Raw(name_w.as_ptr(), is_dir, &mut out_stream_raw, ask_extract_mode, get_prop)
                .and_then(|| Type::from_abi(out_stream_raw))
        }
    }

    fn PrepareOperation7(&self, ask_extract_mode: i32) -> Result<(), Error> {
        unsafe {
            self.PrepareOperation7_Raw(ask_extract_mode)
                .ok()
        }
    }

    fn SetOperationResult8(&self, result_e_operation_result: i32, encrypted: i32, size: u64) -> Result<(), Error> {
        unsafe {
            self.SetOperationResult8_Raw(result_e_operation_result, encrypted, size)
                .ok()
        }
    }
}
