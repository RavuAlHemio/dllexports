//! New Executable (or Segmented Executable) format.
//!
//! The NE format was introduced with Windows 1.0 and supplanted by PE in Windows NT 3.1 and Windows
//! 95.

use std::collections::BTreeMap;
use std::io::{self, Read, Seek, SeekFrom};

use bitflags::bitflags;


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
    pub module_reference_offsets: Vec<u16>, // [u16; module_reference_table_entries]
    pub imported_names: Vec<Vec<u8>>,
    pub entry_table: Vec<EntryBundle>,
    pub non_resident_name_table: Vec<NameTableEntry>,
}
impl Executable {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        // read the MZ executable
        let mz = crate::mz::Executable::read(reader)?;

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
        let non_resident_name_table_bytes = u16::from_le_bytes(header_buf[30..32].try_into().unwrap());
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

        // read the segment table
        reader.seek(SeekFrom::Start(ne_header_offset + u64::from(segment_table_offset)))?;
        let mut segment_table = Vec::with_capacity(segment_table_entries.into());
        for _ in 0..segment_table_entries {
            let entry = SegmentTableEntry::read(reader)?;
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
        let resident_name_table = NameTableEntry::read_table(reader)?;

        // read the module-reference table
        reader.seek(SeekFrom::Start(ne_header_offset + u64::from(module_reference_table_offset)))?;
        let mut module_reference_offsets = Vec::with_capacity(module_reference_table_entries.into());
        for _ in 0..module_reference_table_entries {
            let mut buf = [0u8; 2];
            reader.read_exact(&mut buf)?;
            let offset = u16::from_le_bytes(buf);
            module_reference_offsets.push(offset);
        }

        // read the imported-name table
        let mut imported_names = Vec::new();
        reader.seek(SeekFrom::Start(ne_header_offset + u64::from(imported_names_table_offset)))?;
        loop {
            let mut length_buf = [0u8];
            reader.read_exact(&mut length_buf)?;
            if length_buf[0] == 0 {
                // end of table
                break;
            }

            let mut buf = vec![0u8; length_buf[0].into()];
            reader.read_exact(&mut buf)?;
            imported_names.push(buf);
        }

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

        // read the nonresident-name table
        reader.seek(SeekFrom::Start(ne_header_offset + u64::from(non_resident_name_table_offset)))?;
        let non_resident_name_table = NameTableEntry::read_table(reader)?;

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
            module_reference_offsets,
            imported_names,
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

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SegmentTableEntry {
    pub logical_sector_offset: u16,
    pub segment_length: u16,
    pub flags: SegmentFlags, // u16
    pub min_allocation_size_bytes: u16,
}
impl SegmentTableEntry {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        let mut buf = [0u8; 8];
        reader.read_exact(&mut buf)?;

        let logical_sector_offset = u16::from_le_bytes(buf[0..2].try_into().unwrap());
        let segment_length = u16::from_le_bytes(buf[2..4].try_into().unwrap());
        let flags = SegmentFlags::from_bits_retain(u16::from_le_bytes(buf[4..6].try_into().unwrap()));
        let min_allocation_size_bytes = u16::from_le_bytes(buf[6..8].try_into().unwrap());

        Ok(Self {
            logical_sector_offset,
            segment_length,
            flags,
            min_allocation_size_bytes,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResourceId {
    Numbered(u16),
    Named(Vec<u8>),
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

            Ok(ResourceId::Named(string))
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
            let value = u16::from_le_bytes(buf);
            if value == 0 {
                // that was it
                break;
            }
            let type_id = ResourceId::from_reader_and_value(reader, value, resource_table_pos)?;

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
                    data,
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
    pub data: Vec<u8>, // [u8; resource_length],
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NameTableEntry {
    // length: u8,
    pub name: Vec<u8>, // [u8; length],
    pub ordinal_number: u16,
}
impl NameTableEntry {
    pub fn read_table<R: Read>(reader: &mut R) -> Result<Vec<Self>, io::Error> {
        let mut table = Vec::new();
        loop {
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
                name,
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
}

impl SegmentFlags {
    pub fn type_only(self) -> Self {
        Self::from_bits_retain(self.bits() & 0x0007)
    }
}
