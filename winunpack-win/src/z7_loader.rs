use std::ffi::c_void;

use windows_sys::core::{GUID, HRESULT};


#[cfg(target_os = "windows")]
mod win {
    use super::CreateObject;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    pub fn get_fn_create_object() -> CreateObject {
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
        create_object
    }
}

#[cfg(target_os = "windows")]
pub use win::get_fn_create_object;


#[cfg(not(target_os = "windows"))]
mod unix {
    use super::CreateObject;
    use std::ffi::{c_char, c_int, c_void};

    #[link(name = "dl")]
    unsafe extern "C" {
        fn dlopen(path: *const c_char, flags: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    }

    pub fn get_fn_create_object() -> CreateObject {
        let dl = unsafe {
            dlopen("7z.so")
        };
        if dl.is_null() {
            panic!("failed to load 7z.so");
        }
        let create_object_raw = unsafe {
            dlsym(dl, "CreateObject")
        };
        if create_object_raw.is_null() {
            panic!("failed to import CreateObject from 7z.so");
        }
        let create_object: CreateObject = unsafe {
            std::mem::transmute(create_object_raw)
        };
        create_object
    }
}

#[cfg(not(target_os = "windows"))]
pub use unix::get_fn_create_object;

type CreateObject = unsafe extern "system" fn(
    cls_id: *const GUID,
    iid: *const GUID,
    out_object: *mut *mut c_void,
) -> HRESULT;
