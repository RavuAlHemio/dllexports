// ordinal zero contains the name/description of the library
if ordinal_opt == 0 {
    return false;
}

if path_parts.len != 1 {
    // file without a path or too deep? nope
    return false;
}

let filename = path_parts[0];
let lower_filename = filename.to_lower();
if lower_filename.len > 3 {
    if lower_filename[1] == ":" && lower_filename[2] == "\\" {
        let is_drive =
            (lower_filename[0] >= 'A' && lower_filename[0] <= 'Z')
            || (lower_filename[0] >= 'a' && lower_filename[0] <= 'z');
        if is_drive {
            // Windows path
            lower_filename.replace("\\", "/");
        }
    }
}

// we have extracted the ISOs into a directory named "_extracted";
// strip the path up to and including this directory
let extracted_pos = lower_filename.index_of("/_extracted/");
if extracted_pos > -1 {
    lower_filename.crop(extracted_pos+"/_extracted/".len..);
}
if extracted_pos == -1 && lower_filename.starts_with("_extracted/") {
    lower_filename.crop("_extracted/".len..);
}

if lower_filename.ends_with(".exe") {
    // probably skip this
    // however, there are some DLLs that are named .exe for historical reasons
    if !lower_filename.ends_with("/user.exe") // Win16 GUI API
        && !lower_filename.ends_with("/gdi.exe") // Win16 graphics API
        && !lower_filename.ends_with("/krnl286.exe") // Win16 kernel API for 286 processors
        && !lower_filename.ends_with("/krnl386.exe") // Win16 kernel API for 386 processors
        && !lower_filename.ends_with("/ntoskrnl.exe") // Win32 kernel, single CPU, no physical address extensions
        && !lower_filename.ends_with("/ntkrnlmp.exe") // Win32 kernel, multi-CPU, no physical address extensions
        && !lower_filename.ends_with("/ntkrnlpa.exe") // Win32 kernel, single CPU, physical address extensions
        && !lower_filename.ends_with("/ntkrpamp.exe") // Win32 kernel, multi-CPU, physical address extensions
    {
        // nope, we got an actual application
        return false;
    }
} else if !lower_filename.ends_with(".dll") {
    // not a DLL => not interesting
    return false;
}

let filename_pieces = lower_filename.split("/");

// the first piece is the operating system
// any other piece we must judge accordingly
if filename_pieces.len == 2 {
    // okay; no change required
} else if filename_pieces.len == 3 {
    // 1. Win9x, from Win95 to WinME, stores its DLLs in a subdirectory on the install media
    // 2. WinNT, from WinNT 3.5 to WinXP, stores its DLLs in an architecture-specific subdirectory

    if
        filename_pieces[1] == "win95" // Win95 install media subdirectory
        || filename_pieces[1] == "win98" // Win98 install media subdirectory
        || filename_pieces[1] == "win9x" // WinME install media subdirectory
        || filename_pieces[1] == "i386" // WinNT subdirectory for 32-bit x86 machines
    {
        // remove this subdirectory from the path
        filename_pieces.remove(1);
    } else if
        filename_pieces[1] == "alpha" // WinNT subdirectory for DEC Alpha
        || filename_pieces[1] == "mips" // WinNT subdirectory for MIPS
        || filename_pieces[1] == "ppc" // WinNT subdirectory for PowerPC
    {
        // accept the path, keep the architecture subdirectory intact
    } else {
        // assumed not to be a system component; drop it
        return false;
    }
} else if filename_pieces.len == 4 {
    // WinNT from WinVista onward stores images of the system;
    // expect the DLLs to be under Windows/System32/ (or Windows/SysWow64/ for x86 32-on-64)
    if (filename_pieces[3].starts_with("api-") || filename_pieces[3].starts_with("ext-")) && filename_pieces[3].ends_with(".dll") {
        // API set DLL -- only contains thunks, not implementations
        return false;
    } else if filename_pieces[1] == "windows" || filename_pieces[2] == "system32" {
        filename_pieces.drain(1, 2);
    } else if filename_pieces[1] == "windows" && filename_pieces[2] == "syswow64" {
        // keep syswow64
        filename_pieces.remove(1);
    } else {
        // again, probably not a system DLL
        return false;
    }
} else {
    // no
    return false;
}

filename = join("/", filename_pieces);

true
