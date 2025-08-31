//! Portable Executable format.
//!
//! The PE format was introduced in Windows NT 3.1 and Windows 95; it is based on COFF and used by
//! Windows to this day.

use std::{collections::BTreeMap, io::{self, Read, Seek, SeekFrom}};

use bitflags::bitflags;
use display_bytes::DisplayBytesVec;
use from_to_repr::from_to_other;
use tracing::debug;

use crate::{read_nul_terminated_ascii_string, read_pascal_utf16le_string};


const SEGMENTED_HEADER_OFFSET_OFFSET: u64 = 0x3C;


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Executable {
    pub mz: crate::mz::Executable,

    // follow 32-bit offset at 0x3C to find the following structure:

    // signature: b"PE",
    pub machine_type: MachineType, // u16
    pub section_count: u16,
    pub time_date_stamp: u32,
    pub symbol_table_pointer: u32, // COFF debug info, deprecated
    pub symbol_table_count: u32, // COFF debug info, deprecated
    pub optional_header_size: u16,
    pub characteristics: Characteristics, // u16
    pub optional_header: Option<OptionalHeader>,
    pub section_table: SectionTable,
}
impl Executable {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, io::Error> {
        // read the MZ executable
        let mz = crate::mz::Executable::read(reader)?;

        // I *think* the relocation-data-at-0x0040 prerequisite is no longer true for PE

        // get the offset to the segmented executable header and seek there
        reader.seek(SeekFrom::Start(SEGMENTED_HEADER_OFFSET_OFFSET))?;
        let mut offset_buf = [0u8; 4];
        reader.read_exact(&mut offset_buf)?;
        let pe_header_offset: u64 = u32::from_le_bytes(offset_buf).into();
        reader.seek(SeekFrom::Start(pe_header_offset))?;

        let mut signature_buf = [0u8; 4];
        reader.read_exact(&mut signature_buf)?;
        if &signature_buf != b"PE\0\0" {
            return Err(io::ErrorKind::InvalidData.into());
        }

        let mut header_buf = [0u8; 20];
        reader.read_exact(&mut header_buf)?;

        let machine_type = MachineType::from_base_type(u16::from_le_bytes(header_buf[0..2].try_into().unwrap()));
        let section_count = u16::from_le_bytes(header_buf[2..4].try_into().unwrap());
        let time_date_stamp = u32::from_le_bytes(header_buf[4..8].try_into().unwrap());
        let symbol_table_pointer = u32::from_le_bytes(header_buf[8..12].try_into().unwrap());
        let symbol_table_count = u32::from_le_bytes(header_buf[12..16].try_into().unwrap());
        let optional_header_size = u16::from_le_bytes(header_buf[16..18].try_into().unwrap());
        let characteristics = Characteristics::from_bits_retain(u16::from_le_bytes(header_buf[18..20].try_into().unwrap()));

        let optional_header = OptionalHeader::read(reader, optional_header_size)?;

        // seek past optional header
        reader.seek(SeekFrom::Start(pe_header_offset + 24 + u64::from(optional_header_size)))?;

        let mut section_table_entries = Vec::with_capacity(section_count.into());
        for _ in 0..section_count {
            let entry = SectionTableEntry::read(reader)?;
            section_table_entries.push(entry);
        }
        let mut section_table = SectionTable::from(section_table_entries);
        if let Some(oh) = optional_header.as_ref() {
            if let OptionalHeader::Coff(coff) = oh {
                if let Some(wh) = coff.optional_windows_header.as_ref() {
                    section_table.fix_missing_virtual_sizes(wh.section_alignment);
                }
            }
        }

        Ok(Self {
            mz,
            machine_type,
            section_count,
            time_date_stamp,
            symbol_table_pointer,
            symbol_table_count,
            optional_header_size,
            characteristics,
            optional_header,
            section_table,
        })
    }
}

#[derive(Clone, Copy, Debug)]
#[from_to_other(base_type = u16, derive_compare = "as_int")]
pub enum MachineType {
    Unknown = 0x0000,
    AlphaAxp = 0x0184,
    Alpha64 = 0x0284,
    MatsushitaAm33 = 0x01D3,
    Amd64 = 0x8664,
    Arm = 0x01C0,
    Arm64 = 0xAA64,
    ArmThumb2 = 0x01C4,
    EfiByteCode = 0x0EBC,
    I386 = 0x014C,
    Itanium = 0x0200,
    LoongArch32 = 0x6232,
    LoongArch64 = 0x6264,
    MitsubishiM32r = 0x9041,
    Mips16 = 0x0266,
    MipsWithFpu = 0x0366,
    Mips16WithFpu = 0x0466,
    PowerPc = 0x01F0,
    PowerPcWithFpu = 0x01F1,
    MipsR3kBigEndian = 0x0160,
    MipsR3kLittleEndian = 0x0162,
    MipsR4k = 0x0166,
    MipsR10k = 0x0168,
    RiscV32 = 0x5032,
    RiscV64 = 0x5064,
    RiscV128 = 0x5128,
    HitachiSh3 = 0x01A2,
    HitachiSh3Dsp = 0x01A3,
    HitachiSh4 = 0x01A6,
    HitachiSh5 = 0x01A8,
    ArmThumb = 0x01C2,
    WceMipsV2 = 0x0169,
    Other(u16),
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Characteristics : u16 {
        /// Relocation data has been stripped; image must be loaded at its preferred address.
        ///
        /// Images only, Windows CE or NT only. Only happens by default for EXEs.
        const RELOCS_STRIPPED = 0x0001;

        /// Image is valid and can be run. Lack of this flag indicates a linker error.
        const EXECUTABLE_IMAGE = 0x0002;

        /// COFF line numbers have been stripped.
        ///
        /// This flag is deprecated because COFF debug information is deprecated.
        const LINE_NUMS_STRIPPED = 0x0004;

        /// COFF local symbols have been stripped.
        ///
        /// This flag is deprecated because COFF debug information is deprecated.
        const LOCAL_SYMS_STRIPPED = 0x0008;

        /// Aggressively trim working set.
        ///
        /// Deprecated since Windows 2000 (NT 5.0).
        const AGGRESSIVE_WS_TRIM = 0x0010;

        /// Application can handle addresses beyond 2GB.
        ///
        /// 32-bit applications can theoretically handle up to 4GB, but not if they process memory
        /// addresses using signed integers.
        const LARGE_ADDRESS_AWARE = 0x0020;

        // 0x0040 reserved

        /// Little-endian integers on a big-endian machine. Deprecated.
        const BYTES_REVERSED_LO = 0x0080;

        /// Machine is based on a 32-bit word architecture.
        const IS_32BIT_MACHINE = 0x0100;

        /// Debugging information has been stripped.
        const DEBUG_STRIPPED = 0x0200;

        /// If run from a removable device, copy to swap and run from there.
        const REMOVABLE_RUN_FROM_SWAP = 0x0400;

        /// If run from a network path, copy to swap and run from there.
        const NET_RUN_FROM_SWAP = 0x0800;

        /// Image is a system file (e.g. driver) and not a user program.
        const SYSTEM = 0x1000;

        /// Image is a dynamic-link library.
        const DLL = 0x2000;

        /// Should only be run on uniprocessor machines.
        const UP_SYSTEM_ONLY = 0x4000;

        /// Big-endian integers on little-endian machines. Deprecated.
        const BYTES_REVERSED_HI  = 0x8000;
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum OptionalHeader {
    Coff(OptionalCoffHeader),
    Other { magic: u16, data: Vec<u8> },
}
impl OptionalHeader {
    pub fn read<R: Read + Seek>(reader: &mut R, optional_header_size: u16) -> Result<Option<Self>, io::Error> {
        // optional header cases:
        // * PE32 without Windows header: 28 bytes
        // * PE32+ without Windows header: 24 bytes
        // * PE32 with Windows header: 96 bytes
        // * PE32+ with Windows header: 112 bytes
        // possibly followed by data directories

        if optional_header_size < 2 {
            // not enough for even the magic value
            return Ok(None);
        }

        // find out what kind of optional header we're dealing with
        let mut opt_header_type_buf = [0u8; 2];
        reader.read_exact(&mut opt_header_type_buf)?;
        let opt_header_type = u16::from_le_bytes(opt_header_type_buf);

        let (coff_size, has_base_of_data) = match opt_header_type {
            0x010B => (28, true), // PE32
            0x020B => (24, false), // PE32+
            other_magic => {
                let data_size: usize = (optional_header_size - 2).into();
                let mut data_buf = vec![0u8; data_size];
                reader.read_exact(&mut data_buf)?;
                return Ok(Some(Self::Other { magic: other_magic, data: data_buf }));
            },
        };

        if optional_header_size < coff_size {
            // not big enough for the COFF header
            let data_size: usize = (optional_header_size - 2).into();
            let mut data_buf = vec![0u8; data_size];
            reader.read_exact(&mut data_buf)?;
            return Ok(Some(Self::Other { magic: opt_header_type, data: data_buf }));
        }

        let mut coff_buf = vec![0u8; (coff_size - 2).into()];
        reader.read_exact(&mut coff_buf)?;

        let major_linker_version = coff_buf[0];
        let minor_linker_version = coff_buf[1];
        let code_size = u32::from_le_bytes(coff_buf[2..6].try_into().unwrap());
        let initialized_data_size = u32::from_le_bytes(coff_buf[6..10].try_into().unwrap());
        let uninitialized_data_size = u32::from_le_bytes(coff_buf[10..14].try_into().unwrap());
        let entry_point_addr = u32::from_le_bytes(coff_buf[10..14].try_into().unwrap());
        let base_of_code = u32::from_le_bytes(coff_buf[14..18].try_into().unwrap());

        let base_of_data = if has_base_of_data {
            Some(u32::from_le_bytes(coff_buf[18..22].try_into().unwrap()))
        } else {
            None
        };

        // what about the Windows header?
        let (is_64, windows_header_read_bytes, windows_header_size_requirement) = match opt_header_type {
            0x010B => (false, 68, 96),
            0x020B => (true, 88, 112),
            _ => unreachable!(),
        };
        let optional_windows_header = if optional_header_size >= windows_header_size_requirement {
            let mut win_buf = vec![0u8; windows_header_read_bytes];
            reader.read_exact(&mut win_buf)?;

            let mut i = 0;
            let image_base = if is_64 {
                let ib = u64::from_le_bytes(win_buf[i..i+8].try_into().unwrap());
                i += 8;
                ib
            } else {
                let ib = u32::from_le_bytes(win_buf[i..i+4].try_into().unwrap()).into();
                i += 4;
                ib
            };
            let section_alignment = u32::from_le_bytes(win_buf[i..i+4].try_into().unwrap());
            i += 4;
            let file_alignment = u32::from_le_bytes(win_buf[i..i+4].try_into().unwrap());
            i += 4;
            let major_os_version = u16::from_le_bytes(win_buf[i..i+2].try_into().unwrap());
            i += 2;
            let minor_os_version = u16::from_le_bytes(win_buf[i..i+2].try_into().unwrap());
            i += 2;
            let major_image_version = u16::from_le_bytes(win_buf[i..i+2].try_into().unwrap());
            i += 2;
            let minor_image_version = u16::from_le_bytes(win_buf[i..i+2].try_into().unwrap());
            i += 2;
            let major_subsystem_version = u16::from_le_bytes(win_buf[i..i+2].try_into().unwrap());
            i += 2;
            let minor_subsystem_version = u16::from_le_bytes(win_buf[i..i+2].try_into().unwrap());
            i += 2;
            let win32_version_value = u32::from_le_bytes(win_buf[i..i+4].try_into().unwrap());
            i += 4;
            let image_size = u32::from_le_bytes(win_buf[i..i+4].try_into().unwrap());
            i += 4;
            let headers_size = u32::from_le_bytes(win_buf[i..i+4].try_into().unwrap());
            i += 4;
            let checksum = u32::from_le_bytes(win_buf[i..i+4].try_into().unwrap());
            i += 4;
            let subsystem = Subsystem::from_base_type(u16::from_le_bytes(win_buf[i..i+2].try_into().unwrap()));
            i += 2;
            let dll_characteristics = DllCharacteristics::from_bits_retain(u16::from_le_bytes(win_buf[i..i+2].try_into().unwrap()));
            i += 2;
            let stack_reserve_size = if is_64 {
                let val = u64::from_le_bytes(win_buf[i..i+8].try_into().unwrap());
                i += 8;
                val
            } else {
                let val = u32::from_le_bytes(win_buf[i..i+4].try_into().unwrap()).into();
                i += 4;
                val
            };
            let stack_commit_size = if is_64 {
                let val = u64::from_le_bytes(win_buf[i..i+8].try_into().unwrap());
                i += 8;
                val
            } else {
                let val = u32::from_le_bytes(win_buf[i..i+4].try_into().unwrap()).into();
                i += 4;
                val
            };
            let heap_reserve_size = if is_64 {
                let val = u64::from_le_bytes(win_buf[i..i+8].try_into().unwrap());
                i += 8;
                val
            } else {
                let val = u32::from_le_bytes(win_buf[i..i+4].try_into().unwrap()).into();
                i += 4;
                val
            };
            let heap_commit_size = if is_64 {
                let val = u64::from_le_bytes(win_buf[i..i+8].try_into().unwrap());
                i += 8;
                val
            } else {
                let val = u32::from_le_bytes(win_buf[i..i+4].try_into().unwrap()).into();
                i += 4;
                val
            };
            let loader_flags = u32::from_le_bytes(win_buf[i..i+4].try_into().unwrap());
            i += 4;
            let data_directory_entry_count = u32::from_le_bytes(win_buf[i..i+4].try_into().unwrap());
            i += 4;
            let _ = i;

            // assemble what we have
            let mut windows_header = OptionalWindowsHeader {
                image_base,
                section_alignment,
                file_alignment,
                major_os_version,
                minor_os_version,
                major_image_version,
                minor_image_version,
                major_subsystem_version,
                minor_subsystem_version,
                win32_version_value,
                image_size,
                headers_size,
                checksum,
                subsystem,
                dll_characteristics,
                stack_reserve_size,
                stack_commit_size,
                heap_reserve_size,
                heap_commit_size,
                loader_flags,
                data_directory_entries: Vec::with_capacity(0),
            };

            // do we have enough space for the data directory entries?
            let data_directory_byte_count = data_directory_entry_count * 8;
            if u32::from(windows_header_size_requirement) + data_directory_byte_count >= u32::from(optional_header_size) {
                // yes; go for it
                windows_header.data_directory_entries
                    .reserve(data_directory_entry_count.try_into().unwrap());

                for _ in 0..data_directory_entry_count {
                    let mut entry_buf = [0u8; 8];
                    reader.read_exact(&mut entry_buf)?;
                    let address = u32::from_le_bytes(entry_buf[0..4].try_into().unwrap());
                    let size = u32::from_le_bytes(entry_buf[4..8].try_into().unwrap());
                    windows_header.data_directory_entries.push(DataDirectoryEntry {
                        address,
                        size,
                    });
                }
            }

            Some(windows_header)
        } else {
            None
        };

        // collect it all
        Ok(Some(Self::Coff(OptionalCoffHeader {
            magic: opt_header_type,
            major_linker_version,
            minor_linker_version,
            code_size,
            initialized_data_size,
            uninitialized_data_size,
            entry_point_addr,
            base_of_code,
            base_of_data,
            optional_windows_header,
        })))
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OptionalCoffHeader {
    pub magic: u16,
    pub major_linker_version: u8,
    pub minor_linker_version: u8,
    pub code_size: u32,
    pub initialized_data_size: u32,
    pub uninitialized_data_size: u32,
    pub entry_point_addr: u32,
    pub base_of_code: u32,
    pub base_of_data: Option<u32>, // PE32 only, not in PE32+

    pub optional_windows_header: Option<OptionalWindowsHeader>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OptionalWindowsHeader {
    pub image_base: u64, // u32 on PE32
    pub section_alignment: u32,
    pub file_alignment: u32,
    pub major_os_version: u16,
    pub minor_os_version: u16,
    pub major_image_version: u16,
    pub minor_image_version: u16,
    pub major_subsystem_version: u16,
    pub minor_subsystem_version: u16,
    pub win32_version_value: u32,
    pub image_size: u32,
    pub headers_size: u32,
    pub checksum: u32,
    pub subsystem: Subsystem, // u16
    pub dll_characteristics: DllCharacteristics, // u16
    pub stack_reserve_size: u64, // u32 on PE32
    pub stack_commit_size: u64, // u32 on PE32
    pub heap_reserve_size: u64, // u32 on PE32
    pub heap_commit_size: u64, // u32 on PE32
    pub loader_flags: u32,
    // data_directory_entry_count: u32,
    pub data_directory_entries: Vec<DataDirectoryEntry>,
}
impl OptionalWindowsHeader {
    pub fn known_data_directory_entry(&self, known_entry: KnownDataDirectoryEntry) -> Option<DataDirectoryEntry> {
        let index: usize = known_entry.into();
        if index < self.data_directory_entries.len() {
            Some(self.data_directory_entries[index])
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug)]
#[from_to_other(base_type = u16, derive_compare = "as_int")]
pub enum Subsystem {
    /// Unknown subsystem.
    Unknown = 0,

    /// Driver or native process (e.g. the boot-time file system integrity check).
    Native = 1,

    /// Windows GUI process.
    WindowsGui = 2,

    /// Windows CLI process.
    WindowsCui = 3,

    // 4 reserved

    /// OS/2 CLI process.
    Os2Cui = 5,

    // 6 reserved (OS/2 GUI process?)

    /// POSIX CLI process.
    PosixCui = 7,

    /// Native Windows 9x driver.
    NativeWindows = 8,

    /// Windows CE GUI process.
    WindowsCeGui = 9,

    /// Extensible Firmware Interface application.
    EfiApplication = 10,

    /// Extensible Firmware Interface driver providing boot services.
    EfiBootServiceDriver = 11,

    /// Extensible Firmware Interface driver providing runtime services.
    EfiRuntimeDriver = 12,

    /// Extensible Firmware Interface ROM image.
    EfiRom = 13,

    /// Xbox executable.
    Xbox = 14,

    /// Windows boot application.
    WindowsBootApplication = 16,

    Other(u16),
}


bitflags! {
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct DllCharacteristics : u16 {
        // 0x0001 through 0x0010 reserved

        /// Image can handle high-entropy 64-bit virtual addresses.
        const HIGH_ENTROPY_VIRTUAL_ADDRESSES = 0x0020;

        /// DLL can be relocated at load time.
        const DYNAMIC_BASE = 0x0040;

        /// Code integrity checks are enforced.
        const FORCE_INTEGRITY = 0x0080;

        /// Image is compatible with the No-Execute flag.
        const NX_COMPATIBILITY = 0x0100;

        /// Image is isolation-aware but do not isolate it.
        const NO_ISOLATION = 0x0200;

        /// No structured-exception handling is used in this image.
        const NO_SEH = 0x0400;

        /// Do not bind this image.
        const NO_BIND = 0x0800;

        /// Image must execute in an AppContainer.
        const APPCONTAINER = 0x1000;

        /// Image is a Windows Driver Model driver.
        const WDM_DRIVER = 0x2000;

        /// Image supports Control Flow Guard.
        const GUARD_CF = 0x4000;

        /// Image is aware of Terminal Services.
        const TERMINAL_SERVER_AWARE = 0x8000;
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DataDirectoryEntry {
    pub address: u32,
    pub size: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum KnownDataDirectoryEntry {
    ExportTable,
    ImportTable,
    ResourceTable,
    ExceptionTable,
    CertificateTable,
    BaseRelocationTable,
    Debug,
    Architecture,
    GlobalPtr,
    TlsTable,
    LoadConfigTable,
    BoundImport,
    ImportAddressTable,
    DelayImportDescriptor,
    ClrRuntimeHeader,
    Reserved15,
}
impl From<KnownDataDirectoryEntry> for usize {
    fn from(value: KnownDataDirectoryEntry) -> Self {
        match value {
            KnownDataDirectoryEntry::ExportTable => 0,
            KnownDataDirectoryEntry::ImportTable => 1,
            KnownDataDirectoryEntry::ResourceTable => 2,
            KnownDataDirectoryEntry::ExceptionTable => 3,
            KnownDataDirectoryEntry::CertificateTable => 4,
            KnownDataDirectoryEntry::BaseRelocationTable => 5,
            KnownDataDirectoryEntry::Debug => 6,
            KnownDataDirectoryEntry::Architecture => 7,
            KnownDataDirectoryEntry::GlobalPtr => 8,
            KnownDataDirectoryEntry::TlsTable => 9,
            KnownDataDirectoryEntry::LoadConfigTable => 10,
            KnownDataDirectoryEntry::BoundImport => 11,
            KnownDataDirectoryEntry::ImportAddressTable => 12,
            KnownDataDirectoryEntry::DelayImportDescriptor => 13,
            KnownDataDirectoryEntry::ClrRuntimeHeader => 14,
            KnownDataDirectoryEntry::Reserved15 => 15,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SectionTable {
    entries: Vec<SectionTableEntry>,
}
impl SectionTable {
    pub fn as_entries(&self) -> &[SectionTableEntry] {
        &self.entries
    }

    pub fn fix_missing_virtual_sizes(&mut self, mut section_alignment: u32) {
        // first, sort by virtual position, then by virtual size, then by name
        self.entries.sort_unstable_by_key(|e| (e.virtual_address, e.virtual_size, e.name));

        // prevent division by zero
        if section_alignment == 0 {
            section_alignment = 1;
        }

        // next, run through them
        for i in 0..self.entries.len() {
            let entry = &self.entries[i];
            let next_entry_opt = self.entries.get(i + 1);
            if entry.virtual_size != 0 {
                // good, not much to do here
                continue;
            }

            // round raw size up to alignment to obtain provisional virtual size
            let mut new_virtual_size = ((entry.raw_data_size + (section_alignment - 1)) / section_alignment) * section_alignment;

            // make sure that doesn't overlap with the next section
            if let Some(next_entry) = next_entry_opt {
                if entry.virtual_address + new_virtual_size > next_entry.virtual_address {
                    new_virtual_size = next_entry.virtual_address - entry.virtual_address;
                }
            }

            // FIXME: additional conflict resolution?

            (&mut self.entries[i]).virtual_size = new_virtual_size;
        }
    }

    pub fn has_overlap(&self) -> bool {
        if self.entries.len() == 0 {
            // empty section table has no overlap
            return false;
        }

        let mut virtual_entry_references: Vec<&SectionTableEntry> = self.entries.iter().collect();
        let mut raw_entry_references: Vec<&SectionTableEntry> = self.entries
            .iter()
            .filter(|er| !er.characteristics.contains(SectionCharacteristics::CONTAINS_UNINITIALIZED_DATA))
            .collect();

        {
            // check overlap of raw (in-file) structure
            raw_entry_references.sort_unstable_by_key(|e| (e.raw_data_pointer, e.raw_data_size));
            let mut iterator = raw_entry_references.iter();
            let mut prev_entry = iterator.next().unwrap();
            while let Some(entry) = iterator.next() {
                if prev_entry.raw_data_pointer + prev_entry.raw_data_size > entry.raw_data_pointer {
                    // overlap!
                    debug!(
                        "raw overlap: previous address {:#010X} + previous size {:#010X} = {:#010X} > next address {:#010X}",
                        prev_entry.raw_data_pointer,
                        prev_entry.raw_data_size,
                        prev_entry.raw_data_pointer + prev_entry.raw_data_size,
                        entry.raw_data_pointer,
                    );
                    return true;
                }
                prev_entry = entry;
            }
        }

        {
            // check overlap of virtual (in-memory) structure
            virtual_entry_references.sort_unstable_by_key(|e| (e.virtual_address, e.virtual_size));
            let mut iterator = virtual_entry_references.iter();
            let mut prev_entry = iterator.next().unwrap();
            while let Some(entry) = iterator.next() {
                if prev_entry.virtual_address + prev_entry.virtual_size > entry.virtual_address {
                    // overlap!
                    debug!(
                        "virtual overlap: previous address {:#010X} + previous size {:#010X} = {:#010X} > next address {:#010X}",
                        prev_entry.virtual_address,
                        prev_entry.virtual_size,
                        prev_entry.virtual_address + prev_entry.virtual_size,
                        entry.virtual_address,
                    );
                    return true;
                }
                prev_entry = entry;
            }
        }

        false
    }

    pub fn virtual_to_raw(&self, virtual_addr: u32) -> Option<u32> {
        for entry in &self.entries {
            if virtual_addr >= entry.virtual_address && virtual_addr < entry.virtual_address + entry.virtual_size {
                let offset = virtual_addr - entry.virtual_address;
                if offset >= entry.raw_data_size {
                    // that won't fit
                    return None;
                }
                return Some(entry.raw_data_pointer + offset);
            }
        }
        None
    }

    pub fn raw_to_virtual(&self, raw_addr: u32) -> Option<u32> {
        for entry in &self.entries {
            if raw_addr >= entry.raw_data_pointer && raw_addr < entry.raw_data_pointer + entry.raw_data_size {
                let offset = raw_addr - entry.raw_data_pointer;
                if offset >= entry.virtual_size {
                    // that won't fit
                    return None;
                }
                return Some(entry.virtual_address + offset);
            }
        }
        None
    }
}
impl From<Vec<SectionTableEntry>> for SectionTable {
    fn from(value: Vec<SectionTableEntry>) -> Self {
        Self {
            entries: value,
        }
    }
}
impl From<SectionTable> for Vec<SectionTableEntry> {
    fn from(value: SectionTable) -> Self { value.entries }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SectionTableEntry {
    pub name: [u8; 8],
    pub virtual_size: u32,
    pub virtual_address: u32,
    pub raw_data_size: u32,
    pub raw_data_pointer: u32,
    pub relocations_pointer: u32,
    pub line_numbers_pointer: u32,
    pub relocations_count: u16,
    pub line_numbers_count: u16,
    pub characteristics: SectionCharacteristics, // u32
}
impl SectionTableEntry {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self, io::Error> {
        let mut entry_buf = [0u8; 40];
        reader.read_exact(&mut entry_buf)?;

        let name = entry_buf[0..8].try_into().unwrap();
        let virtual_size = u32::from_le_bytes(entry_buf[8..12].try_into().unwrap());
        let virtual_address = u32::from_le_bytes(entry_buf[12..16].try_into().unwrap());
        let raw_data_size = u32::from_le_bytes(entry_buf[16..20].try_into().unwrap());
        let raw_data_pointer = u32::from_le_bytes(entry_buf[20..24].try_into().unwrap());
        let relocations_pointer = u32::from_le_bytes(entry_buf[24..28].try_into().unwrap());
        let line_numbers_pointer = u32::from_le_bytes(entry_buf[28..32].try_into().unwrap());
        let relocations_count = u16::from_le_bytes(entry_buf[32..34].try_into().unwrap());
        let line_numbers_count = u16::from_le_bytes(entry_buf[34..36].try_into().unwrap());
        let characteristics = SectionCharacteristics::from_bits_retain(u32::from_le_bytes(entry_buf[36..40].try_into().unwrap()));

        Ok(Self {
            name,
            virtual_size,
            virtual_address,
            raw_data_size,
            raw_data_pointer,
            relocations_pointer,
            line_numbers_pointer,
            relocations_count,
            line_numbers_count,
            characteristics,
        })
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct SectionCharacteristics : u32 {
        const NO_PAD = 0x0000_0008;
        const CONTAINS_CODE = 0x0000_0020;
        const CONTAINS_INITIALIZED_DATA = 0x0000_0040;
        const CONTAINS_UNINITIALIZED_DATA = 0x0000_0080;
        const LINK_OTHER = 0x0000_0100;
        const LINK_INFO = 0x0000_0200;
        const LINK_REMOVE = 0x0000_0800;
        const LINK_COMMON_DATA = 0x0000_1000;
        const GLOBAL_POINTER_RELATIVE = 0x0000_8000;
        const MEM_PURGEABLE = 0x0000_8000;
        const MEM_LOCKED = 0x0004_0000;
        const MEM_PRELOAD = 0x0008_0000;
        const ALIGN_BYTES_SHIFT_COUNT_1 = 0x0010_0000;
        const ALIGN_BYTES_SHIFT_COUNT_2 = 0x0020_0000;
        const ALIGN_BYTES_SHIFT_COUNT_4 = 0x0040_0000;
        const ALIGN_BYTES_SHIFT_COUNT_8 = 0x0080_0000;
        const LINK_NRELOC_OVFL = 0x0100_0000;
        const MEM_DISCARDABLE = 0x0200_0000;
        const MEM_NOT_CACHED = 0x0400_0000;
        const MEM_NOT_PAGED = 0x0800_0000;
        const MEM_SHARED = 0x1000_0000;
        const MEM_EXECUTE = 0x2000_0000;
        const MEM_READ = 0x4000_0000;
        const MEM_WRITE = 0x8000_0000;
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExportData {
    pub export_flags: u32,
    pub time_date_stamp: u32,
    pub major_version: u16,
    pub minor_version: u16,
    // pub name_rva: u32,
    pub name: String,
    pub ordinal_base: u32,
    // pub address_table_entry_count: u32,
    // pub name_pointer_and_ordinal_table_entry_count: u32,
    // pub address_table_rva: u32,
    // pub name_pointer_rva: u32,
    // pub ordinal_table_rva: u32,
    pub ordinal_to_address: BTreeMap<u32, ExportAddressTableEntry>,
    pub name_to_ordinal: BTreeMap<String, u32>,
}
impl ExportData {
    pub fn read<R: Read + Seek>(reader: &mut R, export_directory_entry: &DataDirectoryEntry, section_table: &SectionTable) -> Result<Self, io::Error> {
        // ensure the sections don't overlap
        if section_table.has_overlap() {
            return Err(io::ErrorKind::InvalidData.into());
        }

        let position = reader.seek(SeekFrom::Current(0))?;

        // go to offset of export directory
        let export_directory_offset = section_table.virtual_to_raw(export_directory_entry.address)
            .ok_or_else(|| io::ErrorKind::InvalidData)?;
        reader.seek(SeekFrom::Start(export_directory_offset.into()))?;

        let mut buf = [0u8; 40];
        reader.read_exact(&mut buf)?;

        let export_flags = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        let time_date_stamp = u32::from_le_bytes(buf[4..8].try_into().unwrap());
        let major_version = u16::from_le_bytes(buf[8..10].try_into().unwrap());
        let minor_version = u16::from_le_bytes(buf[10..12].try_into().unwrap());
        let name_rva = u32::from_le_bytes(buf[12..16].try_into().unwrap());
        let ordinal_base = u32::from_le_bytes(buf[16..20].try_into().unwrap());
        let address_table_entry_count = u32::from_le_bytes(buf[20..24].try_into().unwrap());
        let name_pointer_and_ordinal_table_entry_count = u32::from_le_bytes(buf[24..28].try_into().unwrap());
        let address_table_rva = u32::from_le_bytes(buf[28..32].try_into().unwrap());
        let name_pointer_rva = u32::from_le_bytes(buf[32..36].try_into().unwrap());
        let ordinal_table_rva = u32::from_le_bytes(buf[36..40].try_into().unwrap());

        // start mapping
        let name_offset = section_table.virtual_to_raw(name_rva)
            .ok_or_else(|| io::ErrorKind::InvalidData)?;
        let address_table_offset = section_table.virtual_to_raw(address_table_rva)
            .ok_or_else(|| io::ErrorKind::InvalidData)?;
        let name_pointer_offset = section_table.virtual_to_raw(name_pointer_rva)
            .ok_or_else(|| io::ErrorKind::InvalidData)?;
        let ordinal_table_offset = section_table.virtual_to_raw(ordinal_table_rva)
            .ok_or_else(|| io::ErrorKind::InvalidData)?;

        // read name
        reader.seek(SeekFrom::Start(name_offset.into()))?;
        let name = read_nul_terminated_ascii_string(reader)?;

        // read address table
        reader.seek(SeekFrom::Start(address_table_offset.into()))?;
        let mut ordinal_to_address = BTreeMap::new();
        for relative_ordinal in 0..address_table_entry_count {
            let ordinal = ordinal_base + relative_ordinal;
            let mut address_buf = [0u8; 4];
            reader.read_exact(&mut address_buf)?;
            let address = u32::from_le_bytes(address_buf);

            if address == 0 {
                // skip this entry
                continue;
            } else if address >= export_directory_entry.address && address < export_directory_entry.address + export_directory_entry.size {
                // forwarder
                let addr_pos = section_table.virtual_to_raw(address)
                    .ok_or_else(|| io::ErrorKind::InvalidData)?;

                let addr_table_pos = reader.seek(SeekFrom::Current(0))?;
                reader.seek(SeekFrom::Start(addr_pos.into()))?;
                let target = read_nul_terminated_ascii_string(reader)?;
                reader.seek(SeekFrom::Start(addr_table_pos))?;
                ordinal_to_address.insert(ordinal, ExportAddressTableEntry::Forwarder { target });
            } else {
                // code
                ordinal_to_address.insert(ordinal, ExportAddressTableEntry::Code { code_rva: address });
            }
        }

        // read names
        let mut name_table = Vec::with_capacity(name_pointer_and_ordinal_table_entry_count.try_into().unwrap());
        reader.seek(SeekFrom::Start(name_pointer_offset.into()))?;
        for _ in 0..name_pointer_and_ordinal_table_entry_count {
            let mut address_buf = [0u8; 4];
            reader.read_exact(&mut address_buf)?;
            let address = u32::from_le_bytes(address_buf);
            let offset = section_table.virtual_to_raw(address)
                .ok_or_else(|| io::ErrorKind::InvalidData)?;
            let name_pointer_pos = reader.seek(SeekFrom::Current(0))?;
            reader.seek(SeekFrom::Start(offset.into()))?;
            let name = read_nul_terminated_ascii_string(reader)?;
            reader.seek(SeekFrom::Start(name_pointer_pos))?;
            name_table.push(name);
        }

        // read ordinals for the names
        // (this is necessary because the name table is sorted ASCIIbetically to enable binary searches,
        // so the mapping from index to ordinal must be explicit)
        let mut name_ordinal_table = Vec::with_capacity(name_pointer_and_ordinal_table_entry_count.try_into().unwrap());
        reader.seek(SeekFrom::Start(ordinal_table_offset.into()))?;
        for _ in 0..name_pointer_and_ordinal_table_entry_count {
            let mut relative_ordinal_buf = [0u8; 2];
            reader.read_exact(&mut relative_ordinal_buf)?;
            let relative_ordinal = u16::from_le_bytes(relative_ordinal_buf);
            let ordinal = ordinal_base + u32::from(relative_ordinal);
            name_ordinal_table.push(ordinal);
        }

        // join the preceding two tables
        let name_to_ordinal: BTreeMap<String, u32> = name_table.into_iter()
            .zip(name_ordinal_table.into_iter())
            .collect();

        reader.seek(SeekFrom::Start(position))?;
        Ok(Self {
            export_flags,
            time_date_stamp,
            major_version,
            minor_version,
            name,
            ordinal_base,
            ordinal_to_address,
            name_to_ordinal,
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ExportAddressTableEntry {
    Skip,
    Code { code_rva: u32 },
    Forwarder { target: String },
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResourceDirectoryTable {
    pub characteristics: u32,
    pub timestamp: u32,
    pub major_version: u16,
    pub minor_version: u16,
    // name_entry_count: u16,
    // id_entry_count: u16,
    pub id_to_entry: BTreeMap<ResourceIdentifier, ResourceChild>, // [(ResourceIdentifier, ResourceChild); name_entry_count + id_entry_count]
}
impl ResourceDirectoryTable {
    pub fn read_from_pe<R: Read + Seek>(reader: &mut R, resources_start_virtual: u32, section_table: &SectionTable) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 16];
        reader.read_exact(&mut header_buf)?;

        let characteristics = u32::from_le_bytes(header_buf[0..4].try_into().unwrap());
        let timestamp = u32::from_le_bytes(header_buf[4..8].try_into().unwrap());
        let major_version = u16::from_le_bytes(header_buf[8..10].try_into().unwrap());
        let minor_version = u16::from_le_bytes(header_buf[10..12].try_into().unwrap());
        let name_entry_count = u16::from_le_bytes(header_buf[12..14].try_into().unwrap());
        let id_entry_count = u16::from_le_bytes(header_buf[14..16].try_into().unwrap());

        let total_entry_count = usize::from(name_entry_count) + usize::from(id_entry_count);
        let mut entry_bytes = vec![0u8; total_entry_count * 8];

        // read the bytes of the entries
        reader.read_exact(&mut entry_bytes)?;

        // collect the entries
        let mut id_to_entry = BTreeMap::new();
        let mut rest = entry_bytes.as_slice();

        for _ in 0..name_entry_count {
            let name_offset = u32::from_le_bytes(rest[0..4].try_into().unwrap());
            let value_offset = u32::from_le_bytes(rest[4..8].try_into().unwrap());

            // the name offset should have the top bit set
            if name_offset & 0x8000_0000 == 0 {
                debug!("named resource entry has a name offset {:#010X} without top bit set", name_offset);
                return Err(io::ErrorKind::InvalidData.into());
            }
            let name_position_virtual = resources_start_virtual + (name_offset & 0x7FFF_FFFF);
            let Some(name_position_raw) = section_table.virtual_to_raw(name_position_virtual) else {
                debug!("failed to find entry name raw position for virtual position {:#010X}", name_position_virtual);
                return Err(io::ErrorKind::InvalidData.into());
            };
            reader.seek(SeekFrom::Start(name_position_raw.into()))?;
            let name = read_pascal_utf16le_string(reader)?;

            // decode the data
            let data = ResourceChild::read_from_pe(
                reader,
                resources_start_virtual,
                value_offset,
                section_table,
            )?;

            let old_entry_opt = id_to_entry.insert(
                ResourceIdentifier::Name(name.clone()),
                data,
            );
            if old_entry_opt.is_some() {
                debug!("duplicate resource key {:?}", ResourceIdentifier::Name(name));
                return Err(io::ErrorKind::InvalidData.into());
            }

            rest = &rest[8..];
        }

        for _ in 0..id_entry_count {
            let id = u32::from_le_bytes(rest[0..4].try_into().unwrap());
            let value_offset = u32::from_le_bytes(rest[4..8].try_into().unwrap());

            // decode the data
            let data = ResourceChild::read_from_pe(
                reader,
                resources_start_virtual,
                value_offset,
                section_table,
            )?;

            let old_entry_opt = id_to_entry.insert(
                ResourceIdentifier::Integer(id),
                data,
            );
            if old_entry_opt.is_some() {
                debug!("duplicate resource key {:?}", ResourceIdentifier::Integer(id));
                return Err(io::ErrorKind::InvalidData.into());
            }

            rest = &rest[8..];
        }

        Ok(Self {
            characteristics,
            timestamp,
            major_version,
            minor_version,
            id_to_entry,
        })
    }

    pub fn read_root_from_pe<R: Read + Seek>(reader: &mut R, resource_table_directory_entry: &DataDirectoryEntry, section_table: &SectionTable) -> Result<Self, io::Error> {
        // ensure the sections don't overlap
        if section_table.has_overlap() {
            return Err(io::ErrorKind::InvalidData.into());
        }

        let position = reader.seek(SeekFrom::Current(0))?;

        // go to offset of resource table directory
        let resource_table_directory_offset = section_table.virtual_to_raw(resource_table_directory_entry.address)
            .ok_or_else(|| io::ErrorKind::InvalidData)?;
        reader.seek(SeekFrom::Start(resource_table_directory_offset.into()))?;

        // recursively read the topmost table
        let ret = Self::read_from_pe(reader, resource_table_directory_entry.address, section_table)?;

        // return to original position
        reader.seek(SeekFrom::Start(position))?;

        // done
        Ok(ret)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ResourceIdentifier {
    Name(String), // name_offset: u32 -> Pascal UTF-16LE string
    Integer(u32),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResourceChild {
    Data(ResourceData),
    Subdirectory(ResourceDirectoryTable),
}
impl ResourceChild {
    pub fn read_from_pe<R: Read + Seek>(reader: &mut R, resources_start_virtual: u32, value_offset_virtual: u32, section_table: &SectionTable) -> Result<Self, io::Error> {
        // check the top bit of the value offset to see if this is a data or subdirectory node
        if value_offset_virtual & 0x8000_0000 == 0 {
            // data
            let data_loc_virtual = resources_start_virtual + value_offset_virtual;
            let Some(data_loc_raw) = section_table.virtual_to_raw(data_loc_virtual) else {
                debug!("failed to find entry value raw position for virtual position {:#010X}", data_loc_virtual);
                return Err(io::ErrorKind::InvalidData.into());
            };
            reader.seek(SeekFrom::Start(data_loc_raw.into()))?;
            let data = ResourceData::read_from_pe(reader, section_table)?;
            Ok(Self::Data(data))
        } else {
            // subdirectory
            let subdir_loc_virtual = resources_start_virtual + (value_offset_virtual & 0x7FFF_FFFF);
            let Some(subdir_loc_raw) = section_table.virtual_to_raw(subdir_loc_virtual) else {
                debug!("failed to find entry value raw position for virtual position {:#010X}", subdir_loc_virtual);
                return Err(io::ErrorKind::InvalidData.into());
            };
            reader.seek(SeekFrom::Start(subdir_loc_raw.into()))?;
            let subdir = ResourceDirectoryTable::read_from_pe(reader, resources_start_virtual, section_table)?;
            Ok(Self::Subdirectory(subdir))
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResourceData {
    pub data_rva: u32,
    pub size: u32,
    pub codepage: u32,
    pub reserved: u32,
    pub data: Option<DisplayBytesVec>, // size bytes at data_rva; None if loading fails
}
impl ResourceData {
    pub fn read_from_pe<R: Read + Seek>(reader: &mut R, section_table: &SectionTable) -> Result<Self, io::Error> {
        let mut header_buf = [0u8; 16];
        reader.read_exact(&mut header_buf)?;

        let data_rva = u32::from_le_bytes(header_buf[0..4].try_into().unwrap());
        let size = u32::from_le_bytes(header_buf[4..8].try_into().unwrap());
        let codepage = u32::from_le_bytes(header_buf[8..12].try_into().unwrap());
        let reserved = u32::from_le_bytes(header_buf[12..16].try_into().unwrap());

        // try our luck
        let mut data = None;
        if let Some(data_raw) = section_table.virtual_to_raw(data_rva) {
            if let Ok(_) = reader.seek(SeekFrom::Start(data_raw.into())) {
                if let Ok(size_usize) = usize::try_from(size) {
                    let mut buf = vec![0u8; size_usize];
                    if let Ok(_) = reader.read_exact(&mut buf) {
                        data = Some(DisplayBytesVec::from(buf));
                    }
                }
            }
        }

        Ok(Self {
            data_rva,
            size,
            codepage,
            reserved,
            data,
        })
    }
}
