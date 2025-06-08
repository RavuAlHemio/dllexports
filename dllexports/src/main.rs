mod data_mgmt;
mod formats;


use std::fs::File;
use std::io::{Cursor, Seek, SeekFrom};
use std::path::PathBuf;

use clap::Parser;
use expandms::fat::{AllocationTable, FatHeader, RootDirectoryLocation};


#[derive(Parser)]
enum ProgMode {
    Expand(ExpandArgs),
    FatHeader(InputFileOnlyArgs),
    FatDirectory(InputFileAndOptIndexArgs),
    FatData(InputFileAndIndexArgs),
    MzHeader(InputFileOnlyArgs),
    NeHeader(InputFileOnlyArgs),
    Interpret(InputFileOnlyArgs),
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


fn main() {
    let mode = ProgMode::parse();
    match mode {
        ProgMode::Expand(args) => {
            let mut input_file = File::open(&args.input_file)
                .expect("failed to open input file");
            let mut output = Vec::new();
            expandms::decompress(&mut input_file, &mut output)
                .expect("failed to decompress");
            std::fs::write(&args.output_file, &output)
                .expect("failed to write output file");
        },
        ProgMode::FatHeader(args) => {
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
        ProgMode::FatDirectory(args) => {
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
        ProgMode::FatData(args) => {
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
        ProgMode::MzHeader(args) => {
            let mut input_file = File::open(&args.input_file)
                .expect("failed to open input file");

            let mz = binms::mz::Executable::read(&mut input_file)
                .expect("failed to read MZ header");
            println!("{:#?}", mz);
        },
        ProgMode::NeHeader(args) => {
            let mut input_file = File::open(&args.input_file)
                .expect("failed to open input file");

            let ne = binms::ne::Executable::read(&mut input_file)
                .expect("failed to read NE header");
            println!("{:#?}", ne);
        },
        ProgMode::Interpret(args) => {
            let input_bytes = std::fs::read(&args.input_file)
                .expect("failed to read input file");
            let interpreted = crate::formats::interpret_file(&input_bytes)
                .expect("failed to interpret input file");
            println!("{:#?}", interpreted);
        },
    }
}
