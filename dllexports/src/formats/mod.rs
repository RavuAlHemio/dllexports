use crate::data_mgmt::{Error, IdentifiedFile};


fn interpret_ne_pe(data: &[u8]) -> Option<Result<IdentifiedFile, Error>> {
    if data.len() < 64 {
        // not NE/PE
        return None;
    }
    let relocation_table_pos = u16::from_le_bytes(data[24..26].try_into().unwrap());
    if relocation_table_pos != 0x0040 {
        // not NE/PE
        return None;
    }
    let ne_pe_pos_u32 = u32::from_le_bytes(data[60..64].try_into().unwrap());
    let ne_pe_pos: usize = ne_pe_pos_u32.try_into().unwrap();
    if ne_pe_pos + 2 > data.len() {
        return None;
    }
    let exe_type = [data[ne_pe_pos], data[ne_pe_pos+1]];
    if &exe_type == b"PE" {
        todo!("parse PE");
    } else if &exe_type == b"NE" {
        todo!("parse NE");
    } else {
        None
    }
}


/// Interprets a file's data.
pub(crate) fn interpret_file(data: &[u8]) -> Result<IdentifiedFile, Error> {
    if data.starts_with(b"MZ") {
        // perhaps NE or PE
        if let Some(ne_pe) = interpret_ne_pe(data) {
            return ne_pe;
        }
    }

    if data.starts_with(b"KWAJ") {
        // single-file KWAJ container
        todo!("KWAJ");
    }

    if data.len() > 2 {
        // starts with a FAT-prescribed jump?
        let looks_like_fat = 
            // jmp short XX, nop
            (data[0] == 0xEB && data[2] == 0x90)
            // jmp near XX
            || data[0] == 0xE9
        ;
        if looks_like_fat {
            todo!("FAT");
        }
    }

    // TODO:
    // * SZDD (single-file container)
    // * CAB (multi-file container)
    // * WIM (m.f.c.)
    // * ISO9660/Joliet (m.f.c.)
    // * possibly NTFS (m.f.c.)

    Ok(IdentifiedFile::Unidentified)
}
