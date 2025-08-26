use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use clap::Parser;
use regex::Regex;
use serde_json;


#[derive(Debug)]
enum IgnoreEntry {
    IgnoreOrdinal(usize),
    AcceptName(Regex),
    IgnoreName(Regex),
    IgnoreSymbol(Regex),
}


#[derive(Parser)]
struct Opts {
    exports_list: PathBuf,
    ignore_list: PathBuf,
}


fn read_ignore_list(path: &Path) -> Vec<IgnoreEntry> {
    // read entries from the ignore list
    let ignore_file = File::open(path)
        .expect("failed to open ignore file");
    let mut ignore_reader = BufReader::new(ignore_file);

    let mut string = String::new();
    let mut ignore_list = Vec::new();
    loop {
        string.clear();
        let bytes_read = ignore_reader.read_line(&mut string)
            .expect("failed to read line");
        if bytes_read == 0 {
             break;
        }

        let trimmed = string.trim();
        if trimmed.len() == 0 {
            // empty line
            continue;
        }
        if trimmed.starts_with("#") {
            // comment
            continue;
        }

        if trimmed.starts_with("@") {
            // ignore ordinal
            let ordinal: usize = trimmed[1..].parse()
                .expect("invalid ordinal");
            ignore_list.push(IgnoreEntry::IgnoreOrdinal(ordinal));
        } else {
            // it's a regex
            let mode = &trimmed[0..1];
            if mode != "+" && mode != "-" && mode != "!" {
                panic!("unknown line type {:?}", trimmed);
            }

            let regex_string = format!("^(?i){}$", &trimmed[1..]);
            let regex = Regex::new(&regex_string)
                .expect("failed to compile regex");
            let entry = match mode {
                "+" => IgnoreEntry::AcceptName(regex),
                "-" => IgnoreEntry::IgnoreName(regex),
                "!" => IgnoreEntry::IgnoreSymbol(regex),
                _ => unreachable!(),
            };
            ignore_list.push(entry);
        }
    }

    ignore_list
}


fn main() {
    let opts = Opts::parse();
    let ignore_list = read_ignore_list(&opts.ignore_list);

    let export_file = File::open(&opts.exports_list)
        .expect("failed to open export file");
    let mut export_reader = BufReader::new(export_file);

    let mut string = String::new();
    loop {
        string.clear();
        let bytes_read = export_reader.read_line(&mut string)
            .expect("failed to read line");
        if bytes_read == 0 {
             break;
        }

        let trimmed = string.trim_end_matches(|c: char| c == '\r' || c == '\n');
        let pieces: Vec<&str> = trimmed.split("\t").collect();
        let path_parts: Vec<String> = serde_json::from_str(&pieces[0])
            .expect("failed to parse path parts");
        let filename_opt = path_parts.last()
            .map(|pp| pp.split(|c| c == '/' || c == '\\').last().unwrap());
        let ordinal_opt: Option<usize> = if pieces[1].len() == 0 {
            None
        } else {
            let ordinal = pieces[1].parse()
                .expect("failed to parse ordinal");
            Some(ordinal)
        };
        let name_opt: Option<&str> = if pieces[2].len() == 0 {
            None
        } else {
            Some(pieces[2])
        };

        // filter?
        let mut output_export = true;
        for ignore_entry in &ignore_list {
            match ignore_entry {
                IgnoreEntry::IgnoreOrdinal(o) => {
                    if let Some(ordinal) = ordinal_opt {
                        if ordinal == *o {
                            output_export = false;
                            break;
                        }
                    }
                },
                IgnoreEntry::AcceptName(r) => {
                    if let Some(file_name) = filename_opt {
                        if r.is_match(file_name) {
                            output_export = true;
                            break;
                        }
                    }
                },
                IgnoreEntry::IgnoreName(r) => {
                    if let Some(file_name) = filename_opt {
                        if r.is_match(file_name) {
                            output_export = false;
                            break;
                        }
                    }
                },
                IgnoreEntry::IgnoreSymbol(r) => {
                    if let Some(name) = name_opt {
                        if r.is_match(name) {
                            output_export = false;
                            break;
                        }
                    }
                },
            }
        }

        if output_export {
            print!("{}", string);
        }
    }
}
