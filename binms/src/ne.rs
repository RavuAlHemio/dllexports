//! New Executable (or Segmented Executable) format.
//!
//! The NE format was introduced with Windows 1.0 and supplanted by PE in Windows NT 3.1 and Windows
//! 95.

use std::collections::BTreeMap;
use std::io::{self, Read, Seek, SeekFrom};

use bitflags::bitflags;
use display_bytes::DisplayBytesVec;
use from_to_repr::{FromToRepr, from_to_other};


const SEGMENTED_HEADER_OFFSET_OFFSET: u64 = 0x3C;


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Executable {
    pub mz: crate::mz::Executable,

    // follow 32-bit offset at 0x3C to find the following structure:
    // (offsets are always relative to start of b"NE" signature)

    // signature: b"NE",
    pub linker_version: u8,
    pub linker_revision: u8,
    // entry_table_offset: u16,
    // entry_table_bytes: u16,
    pub crc32: u32,
    pub flags: ExeFlags, // u16
    pub auto_data_segment_number: u16,
    pub initial_heap_size: u16,
    pub initial_stack_size: u16,
    pub cs_ip: SegmentAndOffset, // u32
    pub ss_sp: SegmentAndOffset, // u32
    // segment_table_entries: u16,
    // module_reference_table_entries: u16,
    // non_resident_name_table_bytes: u16,
    // segment_table_offset: u16,
    // resource_table_offset: u16,
    // resident_name_table_offset: u16,
    // module_reference_table_offset: u16,
    // imported_names_table_offset: u16,
    // non_resident_name_table_offset: u32,
    // entry_table_movable_entries: u16,
    pub logical_sector_alignment_shift_count: u16, // == log_2(segment_sector_size)
    // resource_entries: u16,
    pub executable_type: u8,
    pub reserved: [u8; 9],

    pub segment_table: Vec<SegmentTableEntry>, // [SegmentTableEntry; segment_table_entries]
    pub resource_table: ResourceTable,
    pub resident_name_table: Vec<NameTableEntry>,
    pub entry_table: Vec<EntryBundle>,
    pub non_resident_name_table: Vec<NameTableEntry>,
}
impl Executable {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        // read the MZ executable
        let mz = crate::mz::Executable::read(reader)?;

        // prerequisite for an NE executable: MZ relocation data at 0x0040
        if mz.relocation_table_offset != 0x0040 {
            return Err(io::ErrorKind::InvalidData.into());
        }

        // get the offset to the segmented executable header and seek there
        reader.seek(SeekFrom::Start(SEGMENTED_HEADER_OFFSET_OFFSET))?;
        let mut offset_buf = [0u8; 4];
        reader.read_exact(&mut offset_buf)?;
        let ne_header_offset: u64 = u32::from_le_bytes(offset_buf).into();
        reader.seek(SeekFrom::Start(ne_header_offset))?;

        let mut signature_buf = [0u8; 2];
        reader.read_exact(&mut signature_buf)?;
        if &signature_buf != b"NE" {
            return Err(io::ErrorKind::InvalidData.into());
        }

        let mut header_buf = [0u8; 62];
        reader.read_exact(&mut header_buf)?;

        let linker_version = header_buf[0];
        let linker_revision = header_buf[1];
        let entry_table_offset = u16::from_le_bytes(header_buf[2..4].try_into().unwrap());
        let entry_table_bytes = u16::from_le_bytes(header_buf[4..6].try_into().unwrap());
        let crc32 = u32::from_le_bytes(header_buf[6..10].try_into().unwrap());
        let flags = ExeFlags::from_bits_retain(u16::from_le_bytes(header_buf[10..12].try_into().unwrap()));
        let auto_data_segment_number = u16::from_le_bytes(header_buf[12..14].try_into().unwrap());
        let initial_heap_size = u16::from_le_bytes(header_buf[14..16].try_into().unwrap());
        let initial_stack_size = u16::from_le_bytes(header_buf[16..18].try_into().unwrap());
        let cs_ip = SegmentAndOffset::try_from_slice(&header_buf[18..22]).unwrap();
        let ss_sp = SegmentAndOffset::try_from_slice(&header_buf[22..26]).unwrap();
        let segment_table_entries = u16::from_le_bytes(header_buf[26..28].try_into().unwrap());
        let module_reference_table_entries = u16::from_le_bytes(header_buf[28..30].try_into().unwrap());
        let non_resident_name_table_entries = u16::from_le_bytes(header_buf[30..32].try_into().unwrap());
        let segment_table_offset = u16::from_le_bytes(header_buf[32..34].try_into().unwrap());
        let resource_table_offset = u16::from_le_bytes(header_buf[34..36].try_into().unwrap());
        let resident_name_table_offset = u16::from_le_bytes(header_buf[36..38].try_into().unwrap());
        let module_reference_table_offset = u16::from_le_bytes(header_buf[38..40].try_into().unwrap());
        let imported_names_table_offset = u16::from_le_bytes(header_buf[40..42].try_into().unwrap());
        let non_resident_name_table_offset = u32::from_le_bytes(header_buf[42..46].try_into().unwrap());
        let entry_table_movable_entries = u16::from_le_bytes(header_buf[46..48].try_into().unwrap());
        let logical_sector_alignment_shift_count = u16::from_le_bytes(header_buf[48..50].try_into().unwrap());
        let resource_entries = u16::from_le_bytes(header_buf[50..52].try_into().unwrap());
        let executable_type = header_buf[52];
        let reserved = header_buf[53..62].try_into().unwrap();

        let module_reference_table_absolute_offset = ne_header_offset + u64::from(module_reference_table_offset);
        let imported_names_table_absolute_offset = ne_header_offset + u64::from(imported_names_table_offset);

        // read the segment table
        reader.seek(SeekFrom::Start(ne_header_offset + u64::from(segment_table_offset)))?;
        let mut segment_table = Vec::with_capacity(segment_table_entries.into());
        for _ in 0..segment_table_entries {
            let entry = SegmentTableEntry::read(
                reader,
                logical_sector_alignment_shift_count,
                module_reference_table_absolute_offset,
                imported_names_table_absolute_offset,
            )?;
            segment_table.push(entry);
        }

        // read the resource table
        let resource_table = if resource_table_offset != resident_name_table_offset {
            reader.seek(SeekFrom::Start(ne_header_offset + u64::from(resource_table_offset)))?;
            ResourceTable::read(reader)?
        } else {
            ResourceTable::default()
        };

        // read the resident-name table
        reader.seek(SeekFrom::Start(ne_header_offset + u64::from(resident_name_table_offset)))?;
        let resident_name_table = NameTableEntry::read_table(reader, None)?;

        // read the entry table
        reader.seek(SeekFrom::Start(ne_header_offset + u64::from(entry_table_offset)))?;
        let mut entry_table = Vec::new();
        loop {
            let mut buf1 = [0u8];

            reader.read_exact(&mut buf1)?;
            let entry_count = buf1[0];
            if entry_count == 0 {
                // no more bundles
                break;
            }

            reader.read_exact(&mut buf1)?;
            let segment_indicator = buf1[0];
            let bundle = match segment_indicator {
                0x00 => {
                    // unused entries
                    EntryBundle::Unused { entry_count }
                },
                0xFF => {
                    // moveable segment
                    let mut entries = Vec::with_capacity(entry_count.into());
                    for _ in 0..entry_count {
                        let mut entry_buf = [0u8; 6];
                        reader.read_exact(&mut entry_buf)?;

                        let flags = SegmentEntryFlags::from_bits_retain(entry_buf[0]);
                        let int_3fh = entry_buf[1..3].try_into().unwrap();
                        let segment_number = entry_buf[3];
                        let entry_point_offset = u16::from_le_bytes(entry_buf[4..6].try_into().unwrap());

                        entries.push(MoveableSegmentEntry {
                            flags,
                            int_3fh,
                            segment_number,
                            entry_point_offset,
                        });
                    }
                    EntryBundle::Moveable { entries }
                },
                other => {
                    // fixed segment
                    let mut entries = Vec::with_capacity(entry_count.into());
                    for _ in 0..entry_count {
                        let mut entry_buf = [0u8; 3];
                        reader.read_exact(&mut entry_buf)?;

                        let flags = SegmentEntryFlags::from_bits_retain(entry_buf[0]);
                        let entry_point_offset = u16::from_le_bytes(entry_buf[1..3].try_into().unwrap());

                        entries.push(FixedSegmentEntry {
                            flags,
                            entry_point_offset,
                        });
                    }
                    EntryBundle::Fixed { segment_number: other, entries }
                },
            };
            entry_table.push(bundle);
        }

        // read the nonresident-name table (absolute offset!)
        reader.seek(SeekFrom::Start(non_resident_name_table_offset.into()))?;
        let non_resident_name_table = NameTableEntry::read_table(reader, Some(non_resident_name_table_entries.into()))?;

        Ok(Self {
            mz,
            linker_version,
            linker_revision,
            crc32,
            flags,
            auto_data_segment_number,
            initial_heap_size,
            initial_stack_size,
            cs_ip,
            ss_sp,
            logical_sector_alignment_shift_count,
            executable_type,
            reserved,
            segment_table,
            resource_table,
            resident_name_table,
            entry_table,
            non_resident_name_table,
        })
    }
}


#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SegmentAndOffset {
    pub offset: u16,
    pub segment_number: u16,
}
impl SegmentAndOffset {
    pub fn try_from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() == 4 {
            let offset = u16::from_le_bytes(slice[0..2].try_into().unwrap());
            let segment_number = u16::from_le_bytes(slice[2..4].try_into().unwrap());
            Some(Self {
                offset,
                segment_number,
            })
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SegmentTableEntry {
    pub logical_sector_offset: u16,
    pub segment_length: u16,
    pub flags: SegmentFlags, // u16
    pub min_allocation_size_bytes: u16,
    pub relocation_entries: Vec<RelocationEntry>,
}
impl SegmentTableEntry {
    pub fn read<R: Read + Seek>(
        reader: &mut R,
        logical_sector_alignment_shift_count: u16,
        module_reference_table_absolute_offset: u64,
        imported_names_table_absolute_offset: u64,
    ) -> Result<Self, io::Error> {
        let mut buf = [0u8; 8];
        reader.read_exact(&mut buf)?;

        let logical_sector_offset = u16::from_le_bytes(buf[0..2].try_into().unwrap());
        let segment_length = u16::from_le_bytes(buf[2..4].try_into().unwrap());
        let flags = SegmentFlags::from_bits_retain(u16::from_le_bytes(buf[4..6].try_into().unwrap()));
        let min_allocation_size_bytes = u16::from_le_bytes(buf[6..8].try_into().unwrap());

        let relocation_entries = if flags.contains(SegmentFlags::HAS_RELOCATION_INFO) {
            let segment_table_pos = reader.seek(SeekFrom::Current(0))?;
            let sector_offset = u64::from(logical_sector_offset) * (1 << logical_sector_alignment_shift_count);
            reader.seek(SeekFrom::Start(sector_offset + u64::from(segment_length)))?;

            let mut buf2 = [0u8; 2];
            reader.read_exact(&mut buf2)?;
            let record_number = u16::from_le_bytes(buf2);

            let mut records = Vec::with_capacity(record_number.into());
            for _ in 0..record_number {
                let mut record_buf = [0u8; 8];
                reader.read_exact(&mut record_buf)?;

                let source_type = RelocationEntrySourceType::from_base_type(record_buf[0]);
                let target_and_flags = record_buf[1];
                let source_chain_offset = u16::from_le_bytes(record_buf[2..4].try_into().unwrap());

                let target_type = RelocationEntryTargetType::try_from_repr(target_and_flags & 0b0000_0111).unwrap();
                let flags = RelocationEntryFlags::from_bits_retain(target_and_flags & 0b1111_1000);

                let target = match target_type {
                    RelocationEntryTargetType::InternalReference => {
                        let segment_number = record_buf[4];
                        let zero = record_buf[5];
                        if segment_number == 0xFF {
                            let entry_table_index = u16::from_le_bytes(record_buf[6..8].try_into().unwrap());
                            RelocationTarget::InternalReferenceToMovableSegment {
                                zero,
                                entry_table_index,
                            }
                        } else {
                            let offset_into_segment = u16::from_le_bytes(record_buf[6..8].try_into().unwrap());
                            RelocationTarget::InternalReferenceToFixedSegment {
                                segment_number,
                                zero,
                                offset_into_segment,
                            }
                        }
                    },
                    RelocationEntryTargetType::ImportName => {
                        let module_reference_table_index = u16::from_le_bytes(record_buf[4..6].try_into().unwrap()) - 1;
                        let procedure_imported_names_table_offset = u16::from_le_bytes(record_buf[6..8].try_into().unwrap());

                        // remember where we are
                        let position = reader.seek(SeekFrom::Current(0))?;

                        // to get the module name, we have to follow two references:
                        // &imported_names_table + module_reference_table[module_reference_table_index]
                        // the procedure name is more straightforward:
                        // &imported_names_table + procedure_imported_names_table_offset

                        // go to module reference table entry
                        let module_reference_table_location =
                            module_reference_table_absolute_offset
                            + u64::from(module_reference_table_index) * 2;
                        reader.seek(SeekFrom::Start(module_reference_table_location))?;
                        let mut offset_buf = [0u8; 2];
                        reader.read_exact(&mut offset_buf)?;
                        let module_name_offset = u16::from_le_bytes(offset_buf);

                        // we will need this soon
                        let mut len_buf = [0u8];

                        // read that entry in the imported-names table to get the module name
                        reader.seek(SeekFrom::Start(imported_names_table_absolute_offset + u64::from(module_name_offset)))?;
                        reader.read_exact(&mut len_buf)?;
                        let mut module_name_buf = vec![0u8; len_buf[0].into()];
                        reader.read_exact(&mut module_name_buf)?;

                        // go to the procedure name offset in the import-names table and read it
                        reader.seek(SeekFrom::Start(imported_names_table_absolute_offset + u64::from(procedure_imported_names_table_offset)))?;
                        reader.read_exact(&mut len_buf)?;
                        let mut procedure_name_buf = vec![0u8; len_buf[0].into()];
                        reader.read_exact(&mut procedure_name_buf)?;

                        // seek back
                        reader.seek(SeekFrom::Start(position))?;

                        RelocationTarget::ImportName {
                            module_name: module_name_buf.into(),
                            procedure_name: procedure_name_buf.into(),
                        }
                    },
                    RelocationEntryTargetType::ImportOrdinal => {
                        let module_reference_table_index = u16::from_le_bytes(record_buf[4..6].try_into().unwrap()) - 1;
                        let procedure_ordinal = u16::from_le_bytes(record_buf[6..8].try_into().unwrap());

                        let position = reader.seek(SeekFrom::Current(0))?;

                        let module_reference_table_location =
                            module_reference_table_absolute_offset
                            + u64::from(module_reference_table_index) * 2;
                        reader.seek(SeekFrom::Start(module_reference_table_location))?;
                        let mut offset_buf = [0u8; 2];
                        reader.read_exact(&mut offset_buf)?;
                        let module_name_offset = u16::from_le_bytes(offset_buf);

                        reader.seek(SeekFrom::Start(imported_names_table_absolute_offset + u64::from(module_name_offset)))?;
                        let mut len_buf = [0u8];
                        reader.read_exact(&mut len_buf)?;
                        let mut module_name_buf = vec![0u8; len_buf[0].into()];
                        reader.read_exact(&mut module_name_buf)?;

                        reader.seek(SeekFrom::Start(position))?;

                        RelocationTarget::ImportOrdinal {
                            module_name: module_name_buf.into(),
                            procedure_ordinal,
                        }
                    },
                    RelocationEntryTargetType::OperatingSystemFixup => {
                        let fixup_type = FixupType::from_base_type(u16::from_le_bytes(record_buf[4..6].try_into().unwrap()));
                        let zero = u16::from_le_bytes(record_buf[4..6].try_into().unwrap());
                        RelocationTarget::OperatingSystemFixup {
                            fixup_type,
                            zero,
                        }
                    },
                };

                records.push(RelocationEntry {
                    source_type,
                    flags,
                    source_chain_offset,
                    target,
                });
            }

            reader.seek(SeekFrom::Start(segment_table_pos))?;

            records
        } else {
            Vec::with_capacity(0)
        };

        Ok(Self {
            logical_sector_offset,
            segment_length,
            flags,
            min_allocation_size_bytes,
            relocation_entries,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelocationEntry {
    pub source_type: RelocationEntrySourceType, // u8
    // target_type: RelocationEntryTargetType, // lower 3 bits of u8
    pub flags: RelocationEntryFlags, // upper 5 bits of u8
    pub source_chain_offset: u16,
    pub target: RelocationTarget, // all variants equivalent to [u8; 4]
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RelocationTarget {
    InternalReferenceToFixedSegment {
        segment_number: u8, // except 0xFF
        zero: u8,
        offset_into_segment: u16,
    },
    InternalReferenceToMovableSegment {
        // segment_number == 0xFF: u8,
        zero: u8,
        entry_table_index: u16,
    },
    ImportName {
        // module_reference_table_index: u16,
        // procedure_imported_names_table_offset: u16,
        module_name: DisplayBytesVec, // double-dereferenced
        procedure_name: DisplayBytesVec, // dereferenced
    },
    ImportOrdinal {
        // module_reference_table_index: u16,
        module_name: DisplayBytesVec, // double-dereferenced
        procedure_ordinal: u16,
    },
    OperatingSystemFixup {
        fixup_type: FixupType, // u16
        zero: u16,
    },
}

#[derive(Clone, Copy, Debug)]
#[from_to_other(base_type = u16, derive_compare = "as_int")]
pub enum FixupType {
    FiarqqFjarqq = 0x0001,
    FisrqqFjsrqq = 0x0002,
    FicrqqFjcrqq = 0x0003,
    Fierqq = 0x0004,
    Fidrqq = 0x0005,
    Fiwrqq = 0x0006,
    Other(u16)
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResourceId {
    Numbered(u16),
    Named(DisplayBytesVec),
}
impl ResourceId {
    pub fn from_reader_and_value<R: Read + Seek>(reader: &mut R, value: u16, resource_table_pos: u64) -> Result<Self, io::Error> {
        if value & 0x8000 == 0 {
            // text offset

            // remember where we are
            let return_here_pos = reader.seek(SeekFrom::Current(0))?;

            // go where we are needed
            let text_byte_offset = resource_table_pos + u64::from(value);
            reader.seek(SeekFrom::Start(text_byte_offset))?;

            // 1 byte of length followed by n bytes of string
            let mut buf = [0u8];
            reader.read_exact(&mut buf)?;
            let mut string = vec![0u8; buf[0].into()];
            reader.read_exact(&mut string)?;

            // go back to where we were
            reader.seek(SeekFrom::Start(return_here_pos))?;

            Ok(ResourceId::Named(string.into()))
        } else {
            // number
            Ok(ResourceId::Numbered(value))
        }
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResourceTable {
    pub alignment_shift_count: u16,
    pub id_to_type: BTreeMap<ResourceId, ResourceType>,
}
impl ResourceTable {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let resource_table_pos = reader.seek(SeekFrom::Current(0))?;

        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;
        let alignment_shift_count = u16::from_le_bytes(buf);

        let mut id_to_type = BTreeMap::new();
        loop {
            reader.read_exact(&mut buf)?;
            let type_id_value = u16::from_le_bytes(buf);
            if type_id_value == 0 {
                // that was it
                break;
            }
            let type_id = ResourceId::from_reader_and_value(reader, type_id_value, resource_table_pos)?;

            let mut buf2 = [0u8; 6];
            reader.read_exact(&mut buf2)?;
            let count = u16::from_le_bytes(buf2[0..2].try_into().unwrap());
            let reserved = u32::from_le_bytes(buf2[2..6].try_into().unwrap());

            let mut resources = BTreeMap::new();
            for _ in 0..count {
                let mut resource_buf = [0u8; 12];
                reader.read_exact(&mut resource_buf)?;

                let resource_offset_units = u16::from_le_bytes(resource_buf[0..2].try_into().unwrap());
                let resource_length_units = u16::from_le_bytes(resource_buf[2..4].try_into().unwrap());
                let flags = ResourceFlags::from_bits_retain(u16::from_le_bytes(resource_buf[4..6].try_into().unwrap()));
                let resource_id_value = u16::from_le_bytes(resource_buf[6..8].try_into().unwrap());
                let reserved = u32::from_le_bytes(resource_buf[8..12].try_into().unwrap());

                let file_offset_bytes = u64::from(resource_offset_units) * (1 << alignment_shift_count);
                let resource_length_bytes = usize::from(resource_length_units) * (1 << alignment_shift_count);
                let resource_id = ResourceId::from_reader_and_value(reader, resource_id_value, resource_table_pos)?;

                let location = reader.seek(SeekFrom::Current(0))?;
                reader.seek(SeekFrom::Start(file_offset_bytes))?;
                let mut data = vec![0u8; resource_length_bytes.into()];
                reader.read_exact(&mut data)?;
                reader.seek(SeekFrom::Start(location))?;

                resources.insert(resource_id.clone(), Resource {
                    resource_offset_units,
                    resource_length_units,
                    flags,
                    resource_id,
                    reserved,
                    data: data.into(),
                });
            }

            id_to_type.insert(
                type_id.clone(),
                ResourceType {
                    type_id,
                    reserved,
                    resources,
                },
            );
        }

        Ok(Self {
            alignment_shift_count,
            id_to_type,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResourceType {
    pub type_id: ResourceId,
    // count: u16,
    pub reserved: u32,
    pub resources: BTreeMap<ResourceId, Resource>, // [Resource; count],
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Resource {
    pub resource_offset_units: u16, // (relative to beginning of file, units of (1 << alignment_shift_count))
    pub resource_length_units: u16, // (units of (1 << alignment_shift_count))
    pub flags: ResourceFlags,
    pub resource_id: ResourceId,
    pub reserved: u32,
    pub data: DisplayBytesVec, // [u8; resource_length],
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NameTableEntry {
    // length: u8,
    pub name: DisplayBytesVec, // [u8; length],
    pub ordinal_number: u16,
}
impl NameTableEntry {
    pub fn read_table<R: Read>(reader: &mut R, max_entries: Option<usize>) -> Result<Vec<Self>, io::Error> {
        let mut table = Vec::new();
        loop {
            if let Some(me) = max_entries {
                if table.len() >= me {
                    // we have reached the maximum entry count
                    break;
                }
            }

            let mut length_buf = [0u8];
            reader.read_exact(&mut length_buf)?;
            if length_buf[0] == 0 {
                // end of table
                break;
            }

            let mut name = vec![0u8; length_buf[0].into()];
            reader.read_exact(&mut name)?;

            let mut ordinal_buf = [0u8; 2];
            reader.read_exact(&mut ordinal_buf)?;
            let ordinal_number = u16::from_le_bytes(ordinal_buf);

            table.push(Self {
                name: name.into(),
                ordinal_number,
            })
        }
        Ok(table)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EntryBundle {
    // number_of_entries: u8,
    // segment_indicator: u8, (discriminant)

    Unused {
        // segment_indicator == 0x00
        entry_count: u8,
    },
    Fixed {
        // segment_indicator in 0x01..=0xFE
        segment_number: u8,
        entries: Vec<FixedSegmentEntry>,
    },
    Moveable {
        // segment_indicator == 0xFF
        entries: Vec<MoveableSegmentEntry>,
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FixedSegmentEntry {
    pub flags: SegmentEntryFlags, // u8
    pub entry_point_offset: u16,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MoveableSegmentEntry {
    pub flags: SegmentEntryFlags, // u8
    pub int_3fh: [u8; 2],
    pub segment_number: u8,
    pub entry_point_offset: u16,
}


bitflags! {
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct ExeFlags : u16 {
        const SINGLE_DATA = 0x0001;
        const MULTIPLE_DATA = 0x0002;
        const LINK_ERRORS = 0x2000;
        const LIBRARY_MODULE = 0x8000;
    }

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct SegmentFlags : u16 {
        const DATA = 0x0001;
        const MOVEABLE = 0x0010;
        const PRELOAD = 0x0040;
        const HAS_RELOCATION_INFO = 0x0100;
        const DISCARD = 0xF000;
    }

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct ResourceFlags : u16 {
        const MOVEABLE = 0x0010;
        const PURE = 0x0020;
        const PRELOAD = 0x0040;
    }

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct SegmentEntryFlags : u8 {
        const EXPORTED = 0x01;
        const SHARED_DATA = 0x02;
    }

    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct RelocationEntryFlags : u8 {
        const ADDITIVE = 0x04;
    }
}

impl SegmentFlags {
    pub fn type_only(self) -> Self {
        Self::from_bits_retain(self.bits() & 0x0007)
    }
}

#[derive(Clone, Copy, Debug)]
#[from_to_other(base_type = u8, derive_compare = "as_int")]
pub enum RelocationEntrySourceType {
    LowByte = 0x00,
    Segment = 0x02,
    FarAddress = 0x03,
    Offset = 0x05,
    Other(u8),
}

#[derive(Clone, Copy, Debug, Eq, FromToRepr, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum RelocationEntryTargetType {
    InternalReference = 0x00,
    ImportOrdinal = 0x01,
    ImportName = 0x02,
    OperatingSystemFixup = 0x03,
}
