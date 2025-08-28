mod data_mgmt;
mod formats;
mod read_ext;


use std::fs::{read_dir, File};
use std::io::{Cursor, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, ValueEnum};
use display_bytes::DisplayBytesSlice;
use expandms::fat::{AllocationTable, FatHeader, RootDirectoryLocation};
use expandms::inflate::{Inflater, MAX_LOOKBACK_DISTANCE};
use expandms::iso9660::VolumeDescriptor;
use tracing::{debug, error, info};

use crate::data_mgmt::{IdentifiedFile, PathSequence, Symbol};
use crate::formats::interpret_file;


#[derive(Parser)]
enum ProgMode {
    /// Lower-level file interpretation commands.
    #[command(subcommand)] Poke(PokeMode),

    /// Attempts to ascertain what kind of a file this is.
    Interpret(InputFileOnlyArgs),

    /// Scans a directory and attempts to recursively extract all exports from all exporting files.
    Scan(ScanArgs),
}

#[derive(Parser)]
enum PokeMode {
    /// Expands a file compressed with a Microsoft compression like KWAJ, SZDD or CAB.
    Expand(ExpandArgs),

    /// Obtains low-level information about a File Allocation Table file system.
    #[command(subcommand)] Fat(PokeFatMode),

    /// Obtains low-level information about DOS/Windows executable files.
    #[command(subcommand)] Exe(PokeExeMode),

    /// Obtains low-level information about ISO9660 CD images.
    #[command(subcommand)] Cd(PokeCdMode),

    /// Decompresses DEFLATE-compressed data.
    Inflate(ExpandArgs),
}

#[derive(Parser)]
enum PokeFatMode {
    /// Outputs the header of a File Allocation Table file system.
    FatHeader(InputFileOnlyArgs),

    /// Outputs the entries of a directory in a File Allocation Table file system.
    FatDirectory(InputFileAndOptIndexArgs),

    /// Outputs the data contained in a file in a File Allocation Table file system.
    FatData(InputFileAndIndexArgs),
}

#[derive(Parser)]
enum PokeExeMode {
    /// Outputs the header of an MZ (DOS executable) file.
    MzHeader(InputFileOnlyArgs),

    /// Outputs the header of an NE (16-bit Windows executable) file.
    NeHeader(InputFileOnlyArgs),

    /// Lists icon groups in an NE (16-bit Windows executable) file.
    NeIconGroups(InputFileOnlyArgs),

    /// Outputs icons in an NE (16-bit Windows executable) file.
    NeIcons(InputFileAndGraphicsArgs),
}

#[derive(Parser)]
enum PokeCdMode {
    /// Outputs the first volume descriptor of an ISO9660 or similar image.
    Vol(CdInputFileArgs),
}

#[derive(Parser)]
struct ExpandArgs {
    pub input_file: PathBuf,
    pub output_file: PathBuf,
}

#[derive(Parser)]
struct InputFileOnlyArgs {
    pub input_file: PathBuf,
}

#[derive(Parser)]
struct InputFileAndIndexArgs {
    pub input_file: PathBuf,
    pub index: u32,
}

#[derive(Parser)]
struct InputFileAndOptIndexArgs {
    pub input_file: PathBuf,
    pub index: Option<u32>,
}

#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd, ValueEnum)]
enum GraphicsOutputFormat {
    #[default] Sixel,
    Ascii,
    Png,
}

#[derive(Parser)]
struct InputFileAndGraphicsArgs {
    #[arg(short, long)]
    pub format: GraphicsOutputFormat,

    #[arg(short = 't', long)]
    pub res_type: Option<u16>,

    #[arg(short = 'i', long)]
    pub res_id: Option<u16>,

    pub input_file: PathBuf,
    pub output_file: PathBuf,
}

#[derive(Parser)]
struct ScanArgs {
    pub dir: Option<PathBuf>,
}

#[derive(Parser)]
struct CdInputFileArgs {
    #[arg(short = 'H', long)] pub high_sierra: bool,
    #[arg(short = 'n', long)] pub number: Option<u64>,
    pub input_file: PathBuf,
}


fn set_up_tracing() {
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::fmt;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(std::io::stderr)
        )
        .with(EnvFilter::from_default_env())
        .init();
}


fn escape_name(name: &str) -> String {
    let mut ret = String::with_capacity(name.len());
    for c in name.chars() {
        if c == '\r' {
            ret.push_str("\\r");
        } else if c == '\n' {
            ret.push_str("\\n");
        } else if c == '\t' {
            ret.push_str("\\t");
        } else if c == '\\' {
            ret.push_str("\\\\");
        } else {
            ret.push(c);
        }
    }
    ret
}


fn main() {
    set_up_tracing();

    let mode = ProgMode::parse();
    match mode {
        ProgMode::Poke(poke_mode) => {
            match poke_mode {
                PokeMode::Expand(args) => {
                    let mut input_file = File::open(&args.input_file)
                        .expect("failed to open input file");
                    let mut output = Vec::new();
                    expandms::decompress(&mut input_file, &mut output)
                        .expect("failed to decompress");
                    std::fs::write(&args.output_file, &output)
                        .expect("failed to write output file");
                },
                PokeMode::Fat(poke_fat_mode) => {
                    match poke_fat_mode {
                        PokeFatMode::FatHeader(args) => {
                            let mut input_file = File::open(&args.input_file)
                                .expect("failed to open input file");

                            // read header
                            let fat_header = FatHeader::read(&mut input_file)
                                .expect("failed to read FAT header");
                            println!("{:#?}", fat_header);
                            println!("{:?}", fat_header.variant());

                            // skip over reserved sectors
                            let reserved_bytes = u64::from(fat_header.reserved_sector_count) * u64::from(fat_header.bytes_per_sector);
                            input_file.seek(SeekFrom::Start(reserved_bytes))
                                .expect("failed to seek to start of allocation table");

                            // read allocation table
                            let fat_length = usize::try_from(fat_header.sectors_per_fat).unwrap() * usize::from(fat_header.bytes_per_sector);
                            let allocation_table = AllocationTable::read(&mut input_file, fat_header.variant(), fat_length)
                                .expect("failed to read in allocation table");

                            println!("{:?}", allocation_table);
                        },
                        PokeFatMode::FatDirectory(args) => {
                            let mut input_file = File::open(&args.input_file)
                                .expect("failed to open input file");

                            // read header
                            let fat_header = FatHeader::read(&mut input_file)
                                .expect("failed to read FAT header");

                            // skip over reserved sectors
                            let reserved_bytes = u64::from(fat_header.reserved_sector_count) * u64::from(fat_header.bytes_per_sector);
                            input_file.seek(SeekFrom::Start(reserved_bytes))
                                .expect("failed to seek to start of allocation table");

                            // read allocation table
                            let fat_length = usize::try_from(fat_header.sectors_per_fat).unwrap() * usize::from(fat_header.bytes_per_sector);
                            let allocation_table = AllocationTable::read(&mut input_file, fat_header.variant(), fat_length)
                                .expect("failed to read in allocation table");

                            let mut dir_data = Vec::new();
                            if let Some(subdirectory_cluster_index) = args.index {
                                // read the chain of clusters
                                expandms::fat::read_cluster_chain_into(&mut input_file, &fat_header, &allocation_table, subdirectory_cluster_index, &mut dir_data)
                                    .expect("failed to read cluster chain");
                            } else {
                                match fat_header.root_directory_location {
                                    RootDirectoryLocation::Sector(sector) => {
                                        let sector_count = u32::from(fat_header.max_root_dir_entries) * 32 / u32::from(fat_header.bytes_per_sector);
                                        for i in 0..sector_count {
                                            expandms::fat::read_sector_into(&mut input_file, &fat_header, sector + i, &mut dir_data)
                                                .expect("failed to read sector");
                                        }
                                    },
                                    RootDirectoryLocation::Cluster(cluster) => {
                                        expandms::fat::read_cluster_chain_into(&mut input_file, &fat_header, &allocation_table, cluster, &mut dir_data)
                                            .expect("failed to read cluster chain");
                                    },
                                }
                            }

                            let mut dir_cursor = Cursor::new(&dir_data);
                            let max_entries = dir_data.len() / 32;
                            for _ in 0..max_entries {
                                let entry = expandms::fat::DirectoryEntry::read(&mut dir_cursor, fat_header.variant())
                                    .expect("failed to read directory entry");
                                if entry.file_name[0] == 0x00 {
                                    // no more entries
                                    break;
                                }

                                println!("{:#?}", entry);
                            }
                        },
                        PokeFatMode::FatData(args) => {
                            let mut input_file = File::open(&args.input_file)
                                .expect("failed to open input file");

                            // read header
                            let fat_header = FatHeader::read(&mut input_file)
                                .expect("failed to read FAT header");

                            // skip over reserved sectors
                            let reserved_bytes = u64::from(fat_header.reserved_sector_count) * u64::from(fat_header.bytes_per_sector);
                            input_file.seek(SeekFrom::Start(reserved_bytes))
                                .expect("failed to seek to start of allocation table");

                            // read allocation table
                            let fat_length = usize::try_from(fat_header.sectors_per_fat).unwrap() * usize::from(fat_header.bytes_per_sector);
                            let allocation_table = AllocationTable::read(&mut input_file, fat_header.variant(), fat_length)
                                .expect("failed to read in allocation table");

                            // read a chain of clusters
                            let mut data = Vec::new();
                            expandms::fat::read_cluster_chain_into(&mut input_file, &fat_header, &allocation_table, args.index, &mut data)
                                .expect("failed to read cluster chain");
                            println!("{:?}", data);
                        },
                    }
                },
                PokeMode::Exe(poke_exe_mode) => {
                    match poke_exe_mode {
                        PokeExeMode::MzHeader(args) => {
                            let mut input_file = File::open(&args.input_file)
                                .expect("failed to open input file");

                            let mz = binms::mz::Executable::read(&mut input_file)
                                .expect("failed to read MZ header");
                            println!("{:#?}", mz);
                        },
                        PokeExeMode::NeHeader(args) => {
                            let mut input_file = File::open(&args.input_file)
                                .expect("failed to open input file");

                            let ne = binms::ne::Executable::read(&mut input_file)
                                .expect("failed to read NE header");
                            println!("{:#?}", ne);
                        },
                        PokeExeMode::NeIconGroups(args) => {
                            let mut input_file = File::open(&args.input_file)
                                .expect("failed to open input file");
                            let ne = binms::ne::Executable::read(&mut input_file)
                                .expect("failed to read NE header");

                            for (type_id, res_type) in &ne.resource_table.id_to_type {
                                const CURSOR_LIST: u16 = 0x8000 | 12;
                                const ICON_LIST: u16 = 0x8000 | 14;
                                match type_id {
                                    binms::ne::ResourceId::Numbered(CURSOR_LIST) => {
                                    },
                                    binms::ne::ResourceId::Numbered(ICON_LIST) => {
                                    },
                                    _ => continue,
                                }

                                for (res_id, res) in &res_type.resources {
                                    let data: &[u8] = res.data.as_ref();
                                    let Ok((_rest, icon_group)) = binms::icon_group::IconGroup::take_from_bytes(data)
                                        else { println!("butts"); continue; };
                                    println!("icon group {:?}:", res_id);
                                    for icon in icon_group.icons {
                                        println!("  {:?} ({})", icon, icon.id + 0x8000);
                                    }
                                }
                            }
                        },
                        PokeExeMode::NeIcons(args) => {
                            let mut input_file = File::open(&args.input_file)
                                .expect("failed to open input file");
                            let ne = binms::ne::Executable::read(&mut input_file)
                                .expect("failed to read NE header");

                            for (type_id, res_type) in &ne.resource_table.id_to_type {
                                if let binms::ne::ResourceId::Numbered(type_num) = type_id {
                                    if *type_num == 0x8001 || *type_num == 0x8003 {
                                        if let Some(rt) = args.res_type {
                                            if *type_num != rt {
                                                continue;
                                            }
                                        }

                                        // cursor or icon
                                        for (res_id, res) in &res_type.resources {
                                            if let binms::ne::ResourceId::Numbered(res_num) = res_id {
                                                if let Some(ri) = args.res_id {
                                                    if *res_num != ri {
                                                        continue;
                                                    }
                                                }
                                            }

                                            println!("Resource {:#06X}/{:?}:", type_num, res_id);
                                            let data_bytes: &[u8] = res.data.as_ref();

                                            // try parsing as Ico1
                                            if let Ok((_rest, icon)) = binms::ico1::Icon1::take_from_bytes(data_bytes) {
                                                let variants = [
                                                    ("device-dependent", icon.device_dependent.as_ref()),
                                                    ("device-independent", icon.device_independent.as_ref()),
                                                ];
                                                for (variant_name, variant_icon_opt) in variants {
                                                    let Some(variant_icon) = variant_icon_opt
                                                        else { continue };

                                                    match args.format {
                                                        GraphicsOutputFormat::Sixel => {
                                                            let mut f = File::create(&args.output_file)
                                                                .expect("failed to open output file");

                                                            // spit out the sixel streams
                                                            writeln!(f, "{} AND bytes:", variant_name).unwrap();
                                                            writeln!(f, "{}", variant_icon.and_bytes_as_sixels()).unwrap();
                                                            writeln!(f, "{} XOR bytes:", variant_name).unwrap();
                                                            writeln!(f, "{}", variant_icon.xor_bytes_as_sixels()).unwrap();
                                                            writeln!(f).unwrap();

                                                            f.flush()
                                                                .expect("failed to flush output file");
                                                        },
                                                        GraphicsOutputFormat::Ascii => {
                                                            let mut f = File::create(&args.output_file)
                                                                .expect("failed to open output file");

                                                            // ASCII-only output
                                                            let width = usize::try_from(variant_icon.width_bytes).unwrap();
                                                            for byte_slice in [&variant_icon.and_bytes, &variant_icon.xor_bytes] {
                                                                write!(f, "+").unwrap();
                                                                for _ in 0..8*width {
                                                                    write!(f, "-").unwrap();
                                                                }
                                                                writeln!(f, "+").unwrap();

                                                                for chunk in byte_slice.chunks(width) {
                                                                    write!(f, "|").unwrap();
                                                                    for byte in chunk {
                                                                        for bit in (0..8).rev() {
                                                                            if byte & (1 << bit) != 0 {
                                                                                write!(f, "@").unwrap();
                                                                            } else {
                                                                                write!(f, " ").unwrap();
                                                                            }
                                                                        }
                                                                    }
                                                                    writeln!(f, "|").unwrap();
                                                                }

                                                                write!(f, "+").unwrap();
                                                                for _ in 0..8*width {
                                                                    write!(f, "-").unwrap();
                                                                }
                                                                writeln!(f, "+").unwrap();
                                                            }

                                                            f.flush()
                                                                .expect("failed to flush output file");
                                                        },
                                                        GraphicsOutputFormat::Png => {
                                                            let mut f = File::create(&args.output_file)
                                                                .expect("failed to open output file");

                                                            let width = usize::try_from(variant_icon.width_bytes).unwrap();

                                                            let mut encoder = png::Encoder::new(
                                                                f,
                                                                variant_icon.width_pixels.into(),
                                                                variant_icon.height_pixels.into(),
                                                            );
                                                            encoder.set_color(png::ColorType::Indexed);
                                                            encoder.set_depth(png::BitDepth::Two);

                                                            // palette: 0b00=transparent, 0b01=black, 0b10=white, [0b11=unused]
                                                            encoder.set_palette(&[
                                                                0, 0, 0,
                                                                0, 0, 0,
                                                                255, 255, 255,
                                                            ]);
                                                            // 0b00=transparent, rest opaque
                                                            encoder.set_trns(&[0]);

                                                            let mut writer = encoder.write_header()
                                                                .expect("failed to write PNG header");

                                                            // ensure we do not encode any unused trailing bytes
                                                            let minimum_bytes = (usize::try_from(variant_icon.width_pixels).unwrap() + (8-1)) / 8;

                                                            // now then
                                                            let mut png_data = Vec::new();
                                                            let row_iterator = variant_icon.and_bytes
                                                                .chunks(width)
                                                                .zip(variant_icon.xor_bytes.chunks(width));
                                                            for (and_row, xor_row) in row_iterator {
                                                                let byte_iterator = and_row
                                                                    .iter()
                                                                    .take(minimum_bytes)
                                                                    .copied()
                                                                    .zip(
                                                                        xor_row
                                                                            .iter()
                                                                            .take(minimum_bytes)
                                                                            .copied()
                                                                    );
                                                                for (and_byte, xor_byte) in byte_iterator {
                                                                    let mut word = 0u16;
                                                                    for bit in (0..8).rev() {
                                                                        // if the xor bit is set, make it white
                                                                        // if the and bit is set, make it transparent
                                                                        // otherwise, make it black
                                                                        if xor_byte & (1 << bit) != 0 {
                                                                            word |= 0b10 << (2 * bit);
                                                                        } else if and_byte & (1 << bit) != 0 {
                                                                            word |= 0b00 << (2 * bit);
                                                                        } else {
                                                                            word |= 0b01 << (2 * bit);
                                                                        }
                                                                    }
                                                                    png_data.extend(word.to_be_bytes());
                                                                }
                                                            }
                                                            writer.write_image_data(&png_data)
                                                                .expect("failed to write PNG data");
                                                        },
                                                    }
                                                }
                                            } else if let Ok((_rest, icon)) = binms::bitmap::Bitmap::take_from_bytes(data_bytes, true) {
                                                if args.format != GraphicsOutputFormat::Png {
                                                    println!("can only output as PNG; skipping");
                                                    continue;
                                                }

                                                println!("original bit depth: {}", icon.header.bit_count);

                                                let mut f = File::create(&args.output_file)
                                                    .expect("failed to open output file");

                                                let mut encoder = png::Encoder::new(
                                                    f,
                                                    icon.actual_width(),
                                                    icon.actual_height(),
                                                );

                                                // output as RGBA
                                                // (run optipng on the result if you don't like that)
                                                encoder.set_color(png::ColorType::Rgba);
                                                encoder.set_depth(png::BitDepth::Eight);

                                                let mut writer = encoder.write_header()
                                                    .expect("failed to write PNG header");
                                                writer.write_image_data(&icon.to_rgba8())
                                                    .expect("failed to write PNG data");
                                            } else {
                                                println!("parses as neither ICO1 nor bitmap");
                                            }
                                        }
                                    }
                                }
                            }
                        },
                    }
                },
                PokeMode::Cd(poke_cd_mode) => {
                    match poke_cd_mode {
                        PokeCdMode::Vol(args) => {
                            let mut input_file = File::open(&args.input_file)
                                .expect("failed to open input file");
                            input_file.seek(SeekFrom::Start(0x8000))
                                .expect("failed to seek to volume descriptor");
                            let vd = VolumeDescriptor::read(&mut input_file, args.high_sierra)
                                .expect("failed to read volume descriptor");
                            println!("{:#?}", vd);
                        },
                    }
                },
                PokeMode::Inflate(args) => {
                    let mut input_file = File::open(&args.input_file)
                        .expect("failed to open input file");
                    let mut inflater = Inflater::new(&mut input_file, MAX_LOOKBACK_DISTANCE);
                    let mut output = Vec::new();
                    let mut output_file = File::create(&args.output_file)
                        .expect("failed to create output file");
                    loop {
                        output.clear();
                        let last_block = inflater.inflate_block(&mut output)
                            .expect("failed to inflate block");
                        output_file.write_all(&mut output)
                            .expect("failed to output inflated block to file");
                        if last_block {
                            break;
                        }
                    }
                },
            }
        },
        ProgMode::Interpret(args) => {
            let input_bytes = std::fs::read(&args.input_file)
                .expect("failed to read input file");
            let interpreted = crate::formats::interpret_file(&input_bytes)
                .expect("failed to interpret input file");
            println!("{:#?}", interpreted);
        },
        ProgMode::Scan(args) => {
            // scan the file system recursively
            let dot_path = Path::new(".");
            let top_path = args.dir.as_deref()
                .unwrap_or(dot_path);

            let mut file_list: Vec<PathBuf> = Vec::new();
            let mut dir_stack: Vec<PathBuf> = vec![top_path.to_owned()];
            while let Some(path) = dir_stack.pop() {
                let entries = match read_dir(&path) {
                    Ok(e) => e,
                    Err(e) => {
                        error!("failed to read directory {}: {}", path.display(), e);
                        continue;
                    },
                };

                for entry_res in entries {
                    let entry = match entry_res {
                        Ok(e) => e,
                        Err(e) => {
                            error!("failed to read directory entry from {}: {}", path.display(), e);
                            continue;
                        },
                    };

                    let entry_type = match entry.file_type() {
                        Ok(e) => e,
                        Err(e) => {
                            error!("failed to read type of {}: {}", entry.path().display(), e);
                            continue;
                        },
                    };

                    if entry_type.is_dir() {
                        dir_stack.push(entry.path());
                    } else if entry_type.is_file() {
                        file_list.push(entry.path());
                    }
                }
            }

            // run through the files
            for file_path in file_list {
                let file_data = match std::fs::read(&file_path) {
                    Ok(fd) => fd,
                    Err(e) => {
                        error!("failed to read {}: {}", file_path.display(), e);
                        continue;
                    },
                };
                let path_sequence: PathSequence = vec![file_path].into();
                scan_file(&path_sequence, &file_data);
            }
        },
    }
}


fn scan_file(parent_path_sequence: &PathSequence, data: &[u8]) {
    info!("interpreting {:?}", parent_path_sequence);
    match interpret_file(data) {
        Ok(IdentifiedFile::MultiFileContainer(mfc)) => {
            // scan each child file
            let files = match mfc.list_files() {
                Ok(fs) => fs,
                Err(e) => {
                    error!("failed to list files of {:?}: {}", parent_path_sequence, e);
                    return;
                },
            };
            for file in files {
                let mut child_path_sequence = parent_path_sequence.clone();
                child_path_sequence.push(&file);

                let file_data = match mfc.read_file(&file) {
                    Ok(fd) => {
                        if fd.len() < 24 {
                            debug!("{}", DisplayBytesSlice::from(fd.as_slice()));
                        } else {
                            debug!("{}...{}", DisplayBytesSlice::from(&fd[..16]), DisplayBytesSlice::from(&fd[fd.len()-16..]));
                        }
                        fd
                    },
                    Err(e) => {
                        error!("failed to obtain {:?}: {}", child_path_sequence, e);
                        continue;
                    },
                };
                scan_file(&child_path_sequence, &file_data);
            }
        },
        Ok(IdentifiedFile::SingleFileContainer(sfc)) => {
            let mut child_path_sequence = parent_path_sequence.clone();
            child_path_sequence.push(PathBuf::new());

            let file_data = match sfc.read_file() {
                Ok(fd) => fd,
                Err(e) => {
                    error!("failed to obtain {:?}: {}", child_path_sequence, e);
                    return;
                },
            };
            scan_file(&child_path_sequence, &file_data);
        },
        Ok(IdentifiedFile::SymbolExporter(symex)) => {
            let symbols = match symex.read_symbols() {
                Ok(s) => s,
                Err(e) => {
                    error!("failed to read symbols from {:?}: {}", parent_path_sequence, e);
                    return;
                },
            };
            let path_sequence: &[PathBuf] = parent_path_sequence.as_ref();
            for symbol in symbols {
                match symbol {
                    Symbol::ByName { name }
                        => println!("{:?}\t\t{}", &*path_sequence, escape_name(&name)),
                    Symbol::ByOrdinal { ordinal }
                        => println!("{:?}\t{}\t", &*path_sequence, ordinal),
                    Symbol::ByNameAndOrdinal { name, ordinal }
                        => println!("{:?}\t{}\t{}", &*path_sequence, ordinal, escape_name(&name)),
                }
            }
        },
        Ok(IdentifiedFile::Unidentified) => {
            // guess this one's not that interesting
            return;
        },
        Err(e) => {
            error!("failed to interpret file at {:?}: {}", parent_path_sequence, e);
        },
    }
}
