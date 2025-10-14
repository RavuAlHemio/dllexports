mod extractor;
mod rust_7z_stream;
mod temp_file;


use std::collections::BTreeMap;
use std::ffi::{c_void, OsString};
use std::fs::{File, OpenOptions};
use std::os::windows::ffi::OsStringExt;
use std::os::windows::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::ptr::null_mut;
use std::sync::{Arc, Mutex};

use clap::Parser;
use sxd_document::QName;
use windows::Win32::Foundation::{GENERIC_READ, NTSTATUS};
use windows::Win32::Storage::FileSystem::{FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::System::Variant::{VT_BOOL, VT_BSTR};
use windows_core::{GUID, HRESULT, implement, Interface, s, Type, w};
use z7_com::{
    FormatUdf, FormatWim, IArchiveExtractCallback, IArchiveOpenCallback, IArchiveOpenCallback_Impl,
    IInArchive, IInStream, kExtract, kpidIsDir, kpidPath,
};

use crate::extractor::{MultiFileExtractor, SingleFileExtractor, SingleFileToMemoryExtractor};
use crate::rust_7z_stream::Rust7zInStream;
use crate::temp_file::TempFile;


#[link(name = "ntdll")]
unsafe extern "system" {
    fn RtlGetLastNtStatus() -> NTSTATUS;
}


#[derive(Parser)]
struct Opts {
    pub iso_path: PathBuf,
    pub out_path: PathBuf,
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
            .expect("failed to obtain path property value");
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
    let (install_index, _install_is_esd) = install_index_esd_opt
        .expect("ISO does not contain sources\\install.(wim|esd)");

    // extract it to a temp file
    let wim_temp_file = TempFile::create();
    let wim_path_words = wim_temp_file.path_nul_terminated();
    let wim_path = OsString::from_wide(&wim_path_words[..wim_path_words.len()-1]);
    let temp_writer = wim_temp_file.open_to_write();

    let extractor = SingleFileExtractor::new(
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

    println!("install.(wim|esd) extracted; loading");
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

    // find XML file, Windows/System32/ and Windows/SysWoW64/ files
    let mut xml_file_index_opt: Option<u32> = None;
    let mut paths_indexes: Vec<(String, u32)> = Vec::new();
    for index in 0..item_count {
        // directory? ignore it
        let is_dir_var = unsafe {
            wim_in_archive.GetProperty(index, kpidIsDir.0)
        }
            .expect("failed to obtain is-dir property value");
        assert_eq!(is_dir_var.vt(), VT_BOOL);
        let is_dir = unsafe {
            is_dir_var.Anonymous.Anonymous.Anonymous.boolVal.as_bool()
        };
        if is_dir {
            continue;
        }

        // obtain the path
        let path_var = unsafe {
            wim_in_archive.GetProperty(index, kpidPath.0)
        }
            .expect("failed to obtain path property value");
        assert_eq!(path_var.vt(), VT_BSTR);
        let path_bstr = unsafe {
            &path_var.Anonymous.Anonymous.Anonymous.bstrVal
        };
        let Ok(path_string): Result<String, _> = (&**path_bstr).try_into() else {
            continue;
        };

        let path_lower = path_string.to_lowercase();
        if path_lower == "[1].xml" {
            xml_file_index_opt = Some(index);
        }
        paths_indexes.push((path_string, index));
    }

    let Some(xml_file_index) = xml_file_index_opt else {
        panic!("WIM file does not contain XML definition");
    };

    // find the most interesting Windows variant in the XML file
    let xml_memory_holder: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let xml_extractor = SingleFileToMemoryExtractor::new(
        xml_file_index,
        Arc::clone(&xml_memory_holder),
    );
    let extractor_callback = IArchiveExtractCallback::from(xml_extractor);

    unsafe {
        wim_in_archive.Extract(
            &[xml_file_index],
            kExtract,
            &extractor_callback,
        )
    }
        .expect("failed to extract XML");

    let xml_bytes = {
        let guard = xml_memory_holder
            .lock().expect("failed to lock mutex");
        (*guard).clone()
    };
    let xml_words: Vec<u16> = xml_bytes
        .chunks(2)
        .skip(1)
        .map(|ch| u16::from_le_bytes(ch.try_into().unwrap()))
        .collect();
    let xml_str = String::from_utf16(&xml_words)
        .expect("WIM XML is not UTF-16");
    let xml_pkg = sxd_document::parser::parse(&xml_str)
        .expect("failed to parse WIM XML");
    let image_elems: Vec<_> = xml_pkg
        .as_document()
        .root()
        .children()
        .into_iter()
        .filter_map(|cor| cor.element())
        .nth(0)
        .expect("WIM XML has no root element")
        .children()
        .into_iter()
        .filter_map(|imgn| imgn.element())
        .filter(|imge| imge.name() == QName::new("IMAGE"))
        .collect();

    let mut index_edition = Vec::with_capacity(image_elems.len());
    for image_elem in image_elems {
        let image_index = image_elem.attribute_value("INDEX")
            .expect("<IMAGE> element without INDEX attribute");
        let edition_id: String = image_elem
            .children().into_iter()
            .filter_map(|n| n.element())
            .filter(|e| e.name() == QName::new("WINDOWS"))
            .nth(0)
            .expect("<IMAGE> element without <WINDOWS> child element")
            .children().into_iter()
            .filter_map(|n| n.element())
            .filter(|e| e.name() == QName::new("EDITIONID"))
            .nth(0)
            .expect("<WINDOWS> element without <EDITIONID> child element")
            .children().into_iter()
            .filter_map(|n| n.text())
            .map(|t| t.text())
            .collect();
        index_edition.push((image_index.to_string(), edition_id));
    }

    // editions with most features per version:
    // Vista, 7: "Ultimate"
    // 8, 10, 11: "Professional" (but this has fewer features than "Ultimate" on Vista and 7)
    // => "Ultimate", then "Professional"

    let ultimate_index = index_edition
        .iter()
        .filter(|(_idx, ed)| ed == "Ultimate")
        .map(|(idx, _ed)| idx)
        .nth(0);
    let pro_index = index_edition
        .iter()
        .filter(|(_idx, ed)| ed == "Professional")
        .map(|(idx, _ed)| idx)
        .nth(0);
    let best_index = ultimate_index
        .or(pro_index)
        .expect("found neither Ultimate nor Professional edition");

    // pick out the files that we are interested in
    let wanted_prefixes = vec![
        format!("{}\\windows\\system32\\", best_index),
        format!("{}\\windows\\syswow64\\", best_index),
    ];
    let mut wanted_file_indexes_names = Vec::new();
    for (path, index) in paths_indexes {
        let path_lower = path.to_lowercase();
        let extract_this = wanted_prefixes
            .iter()
            .any(|pfx| path_lower.starts_with(pfx));
        if !extract_this {
            continue;
        }

        if path_lower.ends_with(".wim") {
            continue;
        }

        // strip off the initial chunk (the index)
        let (_index_chunk, extract_path) = path.split_once("\\")
            .expect("split_once failed");
        wanted_file_indexes_names.push((index, extract_path.to_owned()));
    }

    let wanted_file_indexes: Vec<u32> = wanted_file_indexes_names
        .iter()
        .map(|(idx, _path)| *idx)
        .collect();
    let index_to_extract_path: BTreeMap<u32, String> = wanted_file_indexes_names
        .into_iter()
        .collect();

    let extractor = MultiFileExtractor::new(
        opts.out_path.clone(),
        index_to_extract_path,
    );
    let extractor_callback = IArchiveExtractCallback::from(extractor);

    unsafe {
        wim_in_archive.Extract(
            &wanted_file_indexes,
            kExtract,
            &extractor_callback,
        )
    }
        .expect("extraction failed");

    unsafe {
        wim_in_archive.Close()
    }
        .expect("failed to close WIM");

    // delete the WIM temp file
    drop(wim_temp_file);

    unsafe {
        iso_in_archive.Close()
    }
        .expect("failed to close ISO");
}
