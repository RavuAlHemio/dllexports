mod extractor;
mod rust_7z_stream;
mod temp_file;


use std::ffi::{c_void, OsString};
use std::fs::{File, OpenOptions};
use std::os::windows::ffi::OsStringExt;
use std::os::windows::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::ptr::null_mut;
use std::sync::Mutex;

use clap::Parser;
use windows::Win32::Foundation::{GENERIC_READ, NTSTATUS};
use windows::Win32::Storage::FileSystem::{FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::System::Variant::VT_BSTR;
use windows_core::{GUID, HRESULT, implement, Interface, s, Type, w};
use z7_com::{
    FormatUdf, FormatWim, IArchiveExtractCallback, IArchiveOpenCallback, IArchiveOpenCallback_Impl,
    IInArchive, IInStream, kExtract, kpidPath,
};

use crate::extractor::Extractor;
use crate::rust_7z_stream::Rust7zInStream;
use crate::temp_file::TempFile;


#[link(name = "ntdll")]
unsafe extern "system" {
    fn RtlGetLastNtStatus() -> NTSTATUS;
}


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

    // extract it to a temp file
    let wim_temp_file = TempFile::create();
    let wim_path_words = wim_temp_file.path_nul_terminated();
    let wim_path = OsString::from_wide(&wim_path_words[..wim_path_words.len()-1]);
    let temp_writer = wim_temp_file.open_to_write();

    let extractor = Extractor::new(
        install_index,
        temp_writer,
    );
    let extract_callback = IArchiveExtractCallback::from(extractor);

    println!("extracting install.(wim|esd) to {}", wim_path.display());

    unsafe {
        iso_in_archive.Extract(
            &[install_index],
            kExtract,
            &extract_callback,
        )
    }
        .expect("install.(wim|esd) extraction failed");

    println!("install.(wim|esd) extracted");
    drop(extract_callback);

    // load the WIM file now
    let mut wim_in_archive_raw: *mut c_void = null_mut();
    unsafe {
        create_object(
            &FormatWim,
            &IInArchive::IID,
            &mut wim_in_archive_raw,
        )
    }
        .ok()
        .expect("failed to create WIM archive object");
    let wim_in_archive: IInArchive = unsafe {
        Type::from_abi(wim_in_archive_raw)
    }
        .expect("failed to convert WIM archive object");

    let f_res = OpenOptions::new()
        .access_mode(GENERIC_READ.0)
        .share_mode((FILE_SHARE_DELETE | FILE_SHARE_READ | FILE_SHARE_WRITE).0)
        .open(&wim_path);
    let f = match f_res {
        Ok(f) => f,
        Err(e) => {
            // obtain anything
            let lnts = unsafe { RtlGetLastNtStatus() };
            panic!("failed to open WIM file; error: {}; last NT status: {}", e, lnts.0);
        },
    };
    let stream = Rust7zInStream::new(Mutex::new(Box::new(f)));
    unsafe {
        wim_in_archive.Open(
            &IInStream::from(stream),
            None,
            &IArchiveOpenCallback::from(DisinterestedOpenCallback),
        )
    }
        .expect("failed to load WIM");

    let item_count = unsafe {
        wim_in_archive.GetNumberOfItems()
    }
        .expect("failed to obtain number of items");

    println!("WIM item count: {}", item_count);

    // only drop the temp file down here
    drop(wim_temp_file);

    unsafe {
        wim_in_archive.Close()
    }
        .expect("failed to close WIM");

    unsafe {
        iso_in_archive.Close()
    }
        .expect("failed to close ISO");
}
