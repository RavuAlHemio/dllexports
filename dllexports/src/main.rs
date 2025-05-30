use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::path::PathBuf;

use clap::Parser;
use expandms::fat::{AllocationTable, FatHeader};


#[derive(Parser)]
enum ProgMode {
    Expand(ExpandArgs),
    FatHeader(InputFileOnlyArgs),
    FatData(InputFileAndIndexArgs),
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
            let fat_header = FatHeader::read(&mut input_file)
                .expect("failed to read FAT header");
            println!("{:#?}", fat_header);
            println!("{:?}", fat_header.variant());

            // skip over reserved sectors
            let reserved_bytes = u64::from(fat_header.reserved_sector_count) * u64::from(fat_header.bytes_per_sector);
            input_file.seek(SeekFrom::Start(reserved_bytes))
                .expect("failed to seek to start of allocation table");

            let fat_length = usize::try_from(fat_header.sectors_per_fat).unwrap() * usize::from(fat_header.bytes_per_sector);
            let allocation_table = AllocationTable::read(&mut input_file, fat_header.variant(), fat_length)
                .expect("failed to read in allocation table");
            println!("{:?}", allocation_table);
        },
        ProgMode::FatData(args) => {
            let mut input_file = File::open(&args.input_file)
                .expect("failed to open input file");
            let fat_header = FatHeader::read(&mut input_file)
                .expect("failed to read FAT header");

            // skip over reserved sectors
            let reserved_bytes = u64::from(fat_header.reserved_sector_count) * u64::from(fat_header.bytes_per_sector);
            input_file.seek(SeekFrom::Start(reserved_bytes))
                .expect("failed to seek to start of allocation table");

            let fat_length = usize::try_from(fat_header.sectors_per_fat).unwrap() * usize::from(fat_header.bytes_per_sector);
            let allocation_table = AllocationTable::read(&mut input_file, fat_header.variant(), fat_length)
                .expect("failed to read in allocation table");

            let mut data = Vec::new();
            expandms::fat::read_cluster_chain_into(&mut input_file, &fat_header, &allocation_table, args.index, &mut data)
                .expect("failed to read cluster chain");
            println!("{:?}", data);
        },
    }
}
