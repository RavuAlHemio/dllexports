use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use clap::Parser;
use rhai::{Array, Dynamic, Engine, ImmutableString, Scope};
use serde_json;


#[derive(Parser)]
struct Opts {
    exports_list: PathBuf,
    ignore_script: PathBuf,
}


fn remove_prefix(this: ImmutableString, prefix: ImmutableString) -> Dynamic {
    if let Some(stripped) = this.strip_prefix(prefix.as_str()) {
        Dynamic::from(stripped.to_owned())
    } else {
        Dynamic::UNIT
    }
}

fn join(glue: ImmutableString, pieces: Array) -> Dynamic {
    let mut ret = String::new();
    let mut first_piece = true;
    for piece in &pieces {
        if first_piece {
            first_piece = false;
        } else {
            ret.push_str(glue.as_str());
        }

        if let Ok(s) = piece.clone().into_string() {
            ret.push_str(s.as_str());
        } else {
            ret.push_str(&piece.to_string());
        }
    }
    Dynamic::from(ret)
}

fn opt_usize_to_dynamic(ous: Option<usize>) -> Dynamic {
    if let Some(us) = ous {
        Dynamic::from_int(us.try_into().unwrap())
    } else {
        Dynamic::UNIT
    }
}
fn opt_str_to_dynamic<T: Into<String>>(os: Option<T>) -> Dynamic {
    if let Some(s) = os {
        Dynamic::from(s.into())
    } else {
        Dynamic::UNIT
    }
}
fn dynamic_to_opt_usize(dy: Dynamic) -> Option<usize> {
    if dy.is_unit() {
        None
    } else {
        let int_val: i64 = dy.cast();
        Some(int_val.try_into().unwrap())
    }
}
fn dynamic_to_opt_string(dy: Dynamic) -> Option<String> {
    if dy.is_unit() {
        None
    } else {
        Some(dy.cast())
    }
}


fn main() {
    let opts = Opts::parse();

    let mut engine = Engine::new();
    engine.register_fn("remove_prefix", remove_prefix);
    engine.register_fn("join", join);

    let ignore_script = engine.compile_file(opts.ignore_script.clone())
        .expect("failed to compile ignore script");

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
        let filename_opt: Option<String> = path_parts.last()
            .map(|pp| pp
                .split(|c| c == '/' || c == '\\')
                .last().unwrap()
                .to_owned()
            );
        let ordinal_opt: Option<usize> = if pieces[1].len() == 0 {
            None
        } else {
            let ordinal = pieces[1].parse()
                .expect("failed to parse ordinal");
            Some(ordinal)
        };
        let name_opt: Option<String> = if pieces[2].len() == 0 {
            None
        } else {
            Some(pieces[2].to_owned())
        };

        let path_parts_rhai: Array = path_parts.iter()
            .map(|pp| Dynamic::from(pp.clone()))
            .collect();

        let mut scope = Scope::new();
        scope.push("path_parts", path_parts_rhai);
        scope.push("filename_opt", opt_str_to_dynamic(filename_opt));
        scope.push("ordinal_opt", opt_usize_to_dynamic(ordinal_opt));
        scope.push("name_opt", opt_str_to_dynamic(name_opt));

        let output_export: bool = engine.eval_ast_with_scope(&mut scope, &ignore_script)
            .expect("failed to evaluate AST");

        if output_export {
            let new_filename = dynamic_to_opt_string(
                scope.get_value("filename")
                    .expect("filename missing from scope")
            )
                .expect("script did not provide a new filename");
            let new_ordinal_opt = dynamic_to_opt_usize(
                scope.get_value("ordinal_opt")
                    .expect("ordinal_opt missing from scope")
            );
            let new_name_opt = dynamic_to_opt_string(
                scope.get_value("name_opt")
                    .expect("name_opt missing from scope")
            );

            let new_filename_json_list = serde_json::json!([new_filename]);

            print!("{}\t", new_filename_json_list);
            if let Some(new_ordinal) = new_ordinal_opt {
                print!("{}", new_ordinal);
            }
            print!("\t");
            if let Some(new_name) = new_name_opt {
                print!("{}", new_name);
            }
            println!();
        }
    }
}
