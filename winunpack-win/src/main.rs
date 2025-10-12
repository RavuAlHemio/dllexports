mod rust_7z_stream;


use std::{ffi::c_void, sync::Mutex};
use std::fs::File;
use std::path::PathBuf;
use std::ptr::null_mut;

use clap::Parser;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::System::Variant::VT_BSTR;
use windows_core::{GUID, HRESULT, implement, Interface, s, Type, w};
use z7_com::{
    FormatUdf, IArchiveOpenCallback, IArchiveOpenCallback_Impl, IInArchive, IInStream, kpidPath,
};

use crate::rust_7z_stream::Rust7zInStream;


#[derive(Parser)]
struct Opts {
    pub iso_path: PathBuf,
}


type CreateObject = unsafe extern "system" fn(
    cls_id: *const GUID,
    iid: *const GUID,
    out_object: *mut *mut c_void,
) -> HRESULT;


#[implement(IArchiveOpenCallback)]
struct DisinterestedOpenCallback;
impl IArchiveOpenCallback_Impl for DisinterestedOpenCallback_Impl {
    fn SetTotal(&self, files: *const u64, bytes: *const u64) -> windows_core::Result<()> {
        let _ = files;
        let _ = bytes;
        Ok(())
    }

    fn SetCompleted(
        &self,
        files: *const u64,
        bytes: *const u64,
    ) -> windows_core::Result<()> {
        let _ = files;
        let _ = bytes;
        Ok(())
    }
}


fn main() {
    let opts = Opts::parse();

    let hmodule = unsafe {
        LoadLibraryW(w!("7z.dll"))
    }
        .expect("failed to load 7z.dll");
    let create_object_raw = unsafe {
        GetProcAddress(
            hmodule,
            s!("CreateObject"),
        )
    }
        .expect("failed to import CreateObject from 7z.dll");
    let create_object: CreateObject = unsafe {
        std::mem::transmute(create_object_raw)
    };

    // open the ISO (UDF) file
    let mut iso_in_archive_raw: *mut c_void = null_mut();
    unsafe {
        create_object(
            &FormatUdf,
            &IInArchive::IID,
            &mut iso_in_archive_raw,
        )
    }
        .ok()
        .expect("failed to create UDF archive object");
    let iso_in_archive: IInArchive = unsafe {
        Type::from_abi(iso_in_archive_raw)
    }
        .expect("failed to convert UDF archive object");

    let f = File::open(&opts.iso_path)
        .expect("failed to open ISO file");
    let stream = Rust7zInStream::new(Mutex::new(Box::new(f)));
    unsafe {
        iso_in_archive.Open(
            &IInStream::from(stream),
            None,
            &IArchiveOpenCallback::from(DisinterestedOpenCallback),
        )
    }
        .expect("failed to load ISO file as UDF");

    let item_count = unsafe {
        iso_in_archive.GetNumberOfItems()
    }
        .expect("failed to obtain number of items");

    // find sources/install.(esd|wim)
    let mut install_index_esd_opt = None;
    for index in 0..item_count {
        let path_var = unsafe {
            iso_in_archive.GetProperty(index, kpidPath.0)
        }
            .expect("failed to obtain property 0 (path)");
        assert_eq!(path_var.vt(), VT_BSTR);
        let path_bstr = unsafe {
            &path_var.Anonymous.Anonymous.Anonymous.bstrVal
        };
        let Ok(path_string): Result<String, _> = (&**path_bstr).try_into() else {
            continue;
        };
        let path_lower = path_string.to_lowercase();
        if path_lower == "sources\\install.wim" {
            install_index_esd_opt = Some((index, false));
            break;
        } else if path_lower == "sources\\install.esd" {
            install_index_esd_opt = Some((index, true));
            break;
        }
    }
    let (install_index, install_is_esd) = install_index_esd_opt
        .expect("ISO does not contain sources\\install.(wim|esd)");

    unsafe {
        iso_in_archive.Close()
    }
        .expect("failed to close archive");
}
