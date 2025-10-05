mod z7_com;


use std::ffi::c_void;
use std::ptr::null_mut;

use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows_core::{GUID, HRESULT, s, Type, w};

use crate::z7_com::archive::IInArchive;


type CreateObject = unsafe extern "system" fn(
    cls_id: *const GUID,
    iid: *const GUID,
    out_object: *mut *mut c_void,
) -> HRESULT;

const fn msb16(group: u16) -> u8 {
    ((group >> 8) & 0xFF) as u8
}
const fn lsb16(group: u16) -> u8 {
    ((group >> 0) & 0xFF) as u8
}
const fn interface_guid(group: u16, interface: u16) -> GUID {
    GUID::from_values(
        0x23170F69,
        0x40C1,
        0x278A,
        [0x00, 0x00, msb16(group), lsb16(group), msb16(interface), lsb16(interface), 0x00, 0x00],
    )
}
const fn format_guid(format_id: u8) -> GUID {
    GUID::from_values(
        0x23170F69,
        0x40C1,
        0x278A,
        [0x10, 0x00, 0x00, 0x01, 0x10, format_id, 0x00, 0x00],
    )
}
const WIM_FORMAT_GUID: GUID = format_guid(0xE6);
const ISO_FORMAT_GUID: GUID = format_guid(0xE7);


fn main() {
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
        .expect("failed to import CreatObject from 7z.dll");
    let create_object: CreateObject = unsafe {
        std::mem::transmute(create_object_raw)
    };

    const I_OUT_ARCHIVE_GUID: GUID = interface_guid(0x06, 0xA0);

    // open the ISO file
    let mut iso_in_archive_raw: *mut c_void = null_mut();
    unsafe {
        create_object(
            &ISO_FORMAT_GUID,
            &I_OUT_ARCHIVE_GUID,
            &mut iso_in_archive_raw,
        )
    }
        .ok()
        .expect("failed to create ISO archive object");
    let iso_in_archive: IInArchive = unsafe {
        Type::from_abi(iso_in_archive_raw)
    }
        .expect("failed to convert ISO archive object");
    unsafe {
        iso_in_archive.Open(std::ptr::null_mut(), std::ptr::null(), std::ptr::null_mut())
    };
}
