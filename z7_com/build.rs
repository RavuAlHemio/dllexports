use std::path::PathBuf;

use windows_bindgen;


#[cfg(windows)]
macro_rules! path_sep {
    () => ("\\");
}
#[cfg(not(windows))]
macro_rules! path_sep {
    () => ("/");
}


fn main() {
    ensure_winmd();
}

fn ensure_winmd() {
    let mut winmd_path = PathBuf::from(".");
    winmd_path.push("z7_metadata");
    winmd_path.push("IgorPavlov.SevenZip.winmd");

    let curdir = std::env::current_dir()
        .expect("failed to obtain current directory");
    println!("{}", curdir.display());

    match std::fs::exists(&winmd_path) {
        Ok(true) => {},
        Ok(false) => panic!("{} does not exist; please compile it first (see compile.ps1 in that directory)", winmd_path.display()),
        Err(e) => panic!("failed to check for the existence of {}: {}", winmd_path.display(), e),
    }

    let warnings = windows_bindgen::bindgen([
        "--in",
            concat!("z7_metadata", path_sep!(), "IgorPavlov.SevenZip.winmd"),
            concat!("z7_metadata", path_sep!(), "Windows.Win32.winmd"),
        "--reference",
            "windows,skip-root,Windows",
        "--out",
            concat!("src", path_sep!(), "bindings.rs"),
        "--filter",
            "IgorPavlov.SevenZip",
        "--no-deps",
    ]);

    println!("{}", warnings);
}
