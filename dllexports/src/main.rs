use std::fs::File;
use std::path::PathBuf;

use clap::Parser;


#[derive(Parser)]
enum ProgMode {
    Expand(ExpandArgs),
}

#[derive(Parser)]
struct ExpandArgs {
    pub input_file: PathBuf,
    pub output_file: PathBuf,
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
    }
}
