mod cdrom;
mod exe;
mod fat;
mod single_compression;


use std::collections::BTreeMap;
use std::io::Cursor;

use binms::ne::{self, SegmentEntryFlags};
use binms::pe::{self, ExportData, KnownDataDirectoryEntry, OptionalHeader};

use crate::data_mgmt::{Error, IdentifiedFile, Symbol};
use crate::formats::exe::{NewExecutable, PortableExecutable};
use crate::formats::fat::FatFileSystem;
use crate::formats::single_compression::KwajOrSz;


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
        let mut cursor = Cursor::new(data);
        let portable_executable = match pe::Executable::read(&mut cursor) {
            Ok(pe) => pe,
            Err(e) => return Some(Err(Error::Io(e))),
        };

        // export table?
        if let Some(optional_header) = &portable_executable.optional_header {
            if let OptionalHeader::Coff(coff) = &optional_header {
                if let Some(windows) = &coff.optional_windows_header {
                    if let Some(export_directory_entry) = windows.known_data_directory_entry(KnownDataDirectoryEntry::ExportTable) {
                        if export_directory_entry.address == 0 && export_directory_entry.size == 0 {
                            // no exports
                            return Some(Ok(IdentifiedFile::SymbolExporter(Box::new(PortableExecutable {
                                exports: Vec::with_capacity(0),
                            }))));
                        }
                        let export_data_res = ExportData::read(
                            &mut cursor,
                            &export_directory_entry,
                            &portable_executable.section_table,
                        );
                        let export_data = match export_data_res {
                            Ok(ed) => ed,
                            Err(e) => return Some(Err(Error::Io(e))),
                        };

                        // collect symbols according to ordinal
                        let mut ordinal_to_symbol = BTreeMap::new();
                        for ordinal in export_data.ordinal_to_address.keys() {
                            ordinal_to_symbol.insert(
                                *ordinal,
                                Symbol::ByOrdinal { ordinal: *ordinal },
                            );
                        }

                        // enrich with names
                        for (name, ordinal) in &export_data.name_to_ordinal {
                            if let Some(symbol) = ordinal_to_symbol.get_mut(ordinal) {
                                *symbol = Symbol::ByNameAndOrdinal {
                                    name: name.clone(),
                                    ordinal: *ordinal,
                                };
                            }
                        }

                        let exports: Vec<Symbol> = ordinal_to_symbol.into_values().collect();
                        return Some(Ok(IdentifiedFile::SymbolExporter(Box::new(PortableExecutable {
                            exports,
                        }))));
                    }
                }
            }
        }
        return None;
    } else if &exe_type == b"NE" {
        let mut cursor = Cursor::new(data);
        let new_executable = match ne::Executable::read(&mut cursor) {
            Ok(ne) => ne,
            Err(e) => return Some(Err(Error::Io(e))),
        };

        // collect exported entry points and their ordinals
        let mut exports = Vec::new();
        let mut ordinal: u32 = 0;
        for entry in &new_executable.entry_table {
            match entry {
                ne::EntryBundle::Unused { entry_count } => {
                    ordinal += u32::from(*entry_count);
                },
                ne::EntryBundle::Fixed { entries, .. } => {
                    for entry in entries {
                        if entry.flags.contains(SegmentEntryFlags::EXPORTED) {
                            exports.push(Symbol::ByOrdinal {
                                ordinal: ordinal,
                            });
                        }
                        ordinal += 1;
                    }
                },
                ne::EntryBundle::Moveable { entries } => {
                    for entry in entries {
                        if entry.flags.contains(SegmentEntryFlags::EXPORTED) {
                            exports.push(Symbol::ByOrdinal {
                                ordinal: ordinal,
                            });
                        }
                        ordinal += 1;
                    }
                },
            }
        }

        // run through resident and nonresident name tables to enrich ordinals with names
        for table in &[&new_executable.resident_name_table, &new_executable.non_resident_name_table] {
            for entry in *table {
                let name_bytes: &[u8] = entry.name.as_ref();
                let Ok(name) = String::from_utf8(name_bytes.to_vec()) else { continue };

                let symbol_opt = exports.iter_mut()
                    .filter(|s| s.ordinal() == Some(entry.ordinal_number.into()))
                    .nth(0);
                let Some(symbol) = symbol_opt else { continue };
                *symbol = Symbol::ByNameAndOrdinal {
                    name,
                    ordinal: symbol.ordinal().unwrap(),
                };
            }
        }

        Some(Ok(IdentifiedFile::SymbolExporter(Box::new(NewExecutable {
            exports,
        }))))
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

    let is_kwaj_or_sz =
        data.starts_with(b"KWAJ\x88\xF0\x27\xD1")
        || data.starts_with(b"SZDD\x88\xF0\x27\x33")
        || data.starts_with(b"SZ \x88\xF0\x27\x33\xD1");
    if is_kwaj_or_sz {
        // single-file KWAJ, SZDD or SZ container
        return Ok(IdentifiedFile::SingleFileContainer(Box::new(KwajOrSz::new(data.to_vec()))));
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
            let thicc = FatFileSystem::new(data.to_owned())?;
            return Ok(IdentifiedFile::MultiFileContainer(Box::new(thicc)));
        }
    }

    // for CD-ROMs, we need to look a bit further
    if data.len() >= 0x8006 {
        if &data[0x8001..0x8006] == b"CD001" {
            let cd = crate::formats::cdrom::Cdrom::new_from_iso9660_data(data)?;
            return Ok(IdentifiedFile::MultiFileContainer(Box::new(cd)));
        }
    } 
    if data.len() >= 0x800E {
        if &data[0x8009..0x800E] == b"CDROM" {
            let cd = crate::formats::cdrom::Cdrom::new_from_high_sierra_data(data)?;
            return Ok(IdentifiedFile::MultiFileContainer(Box::new(cd)));
        }
    }

    // TODO:
    // * SZDD (single-file container)
    // * CAB (multi-file container)
    // * WIM (m.f.c.)
    // * possibly NTFS (m.f.c.)

    Ok(IdentifiedFile::Unidentified)
}
