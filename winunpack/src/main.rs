mod udf;


use std::ffi::CString;
use std::io::Write;
use std::num::NonZero;
use std::path::PathBuf;

use clap::Parser;
use sxd_document::QName;
use tempfile::NamedTempFile;
use wimlib::{ExtractFlags, IterateDirTreeFlags, OpenFlags, WimLib};
use wimlib::string::{TStr, ThinTStr};

use crate::udf::Udf;


#[derive(Parser)]
struct Opts {
    pub iso_path: PathBuf,
    pub out_path: PathBuf,
}

fn main() {
    let opts = Opts::parse();

    let iso = Udf::open(&opts.iso_path)
        .expect("failed to open UDF");
    let root_entry = iso.get_root(None)
        .expect("failed to get ISO root");
    let sources_dir = root_entry
        .advance_until(|e|
            e.is_dir()
            && e
                .name()
                .map(|n| n.to_bytes().to_ascii_lowercase() == b"sources")
                .unwrap_or(false)
        )
        .expect("failed to find \"sources\" subdirectory");

    // find install.wim
    let sources_entry = sources_dir.descend()
        .expect("failed to descend into \"sources\" subdirectory");
    let mut install_wim_opt = sources_entry
        .advance_until(|e|
            !e.is_dir()
            && e
                .name()
                .map(|n| n.to_bytes().to_ascii_lowercase() == b"install.wim")
                .unwrap_or(false)
        );
    if install_wim_opt.is_none() {
        // try install.esd instead
        let sources_entry = sources_dir.descend()
            .expect("failed to descend into \"sources\" subdirectory");
        install_wim_opt = sources_entry
            .advance_until(|e|
                !e.is_dir()
                && e
                    .name()
                    .map(|n| n.to_bytes().to_ascii_lowercase() == b"install.esd")
                    .unwrap_or(false)
            );
    }
    let mut install_wim = install_wim_opt
        .expect("found neither install.wim nor install.esd");

    // extract WIM to a temp file
    let wim_size_bytes: usize = install_wim
        .file_length().expect("failed to obtain .wim file size")
        .try_into().expect("failed to convert .wim size to usize");
    let mut wim_temp_file_holder = NamedTempFile::new_in(".")
        .expect("failed to create temp file for .wim");
    let wim_path = wim_temp_file_holder.path().to_path_buf();
    let wim_temp_file = wim_temp_file_holder.as_file_mut();

    println!("extracting install.(wim|esd) to {}", wim_path.display());

    let mut buf = vec![0u8; 512*crate::udf::BLOCK_LENGTH];
    let mut total_bytes_read = 0;
    loop {
        if total_bytes_read >= wim_size_bytes {
            break;
        }

        let remaining_bytes = wim_size_bytes - total_bytes_read;
        let blocks_to_read = if remaining_bytes < buf.len() {
            // round up to full blocks though
            remaining_bytes.div_ceil(crate::udf::BLOCK_LENGTH)
        } else {
            buf.len() / crate::udf::BLOCK_LENGTH
        };
        let bytes_to_read = blocks_to_read * crate::udf::BLOCK_LENGTH;
        let bytes_read_isize = install_wim.read(&mut buf[..bytes_to_read]);
        if bytes_read_isize == 0 {
            break;
        } else if bytes_read_isize < 0 {
            panic!("failed to read .wim file from ISO");
        }
        let bytes_read: usize = bytes_read_isize.try_into().unwrap();
        let bytes_to_write = if total_bytes_read + bytes_read > wim_size_bytes {
            wim_size_bytes - total_bytes_read
        } else {
            bytes_read
        };
        total_bytes_read += bytes_read;

        wim_temp_file.write_all(&buf[..bytes_to_write])
            .expect("failed to write install.(wim|esd)");
    }
    wim_temp_file.flush()
        .expect("failed to flush install.(wim|esd)");

    println!("install.(wim|esd) extracted; loading");

    // load the WIM file now
    let wim_lib = WimLib::default();
    let wim_path_c_string = CString::new(wim_path.as_os_str().as_encoded_bytes())
        .expect("WIM path has NULs");
    let wim_path_tstr = TStr::from_impl(&wim_path_c_string);
    let wim = wim_lib.open_wim(wim_path_tstr, OpenFlags::CHECK_INTEGRITY)
        .expect("failed to open WIM file");
    let xml_data = wim.xml_data()
        .expect("failed to obtain WIM XML data");
    let mut xml_data_string = xml_data.to_string()
        .expect("failed to decode WIM XML data as UTF-16");
    if xml_data_string.starts_with('\u{FEFF}') {
        xml_data_string.remove(0);
    }

    // find the most interesting Windows variant in the XML file
    let xml_pkg = sxd_document::parser::parse(&xml_data_string)
        .expect("failed to parse WIM XML");
    let image_elems: Vec<_> = xml_pkg
        .as_document()
        .root()
        .children()
        .into_iter()
        .filter_map(|cor| cor.element())
        .nth(0)
        .expect("WIM XML has no root element")
        .children()
        .into_iter()
        .filter_map(|imgn| imgn.element())
        .filter(|imge| imge.name() == QName::new("IMAGE"))
        .collect();

    let mut index_edition = Vec::with_capacity(image_elems.len());
    for image_elem in image_elems {
        let image_index: u32 = image_elem.attribute_value("INDEX")
            .expect("<IMAGE> element without INDEX attribute")
            .parse().expect("<IMAGE> INDEX attribute not u32");
        let image_index_nz = NonZero::new(image_index)
            .expect("<IMAGE> element INDEX attribute is zero");
        let edition_id: String = image_elem
            .children().into_iter()
            .filter_map(|n| n.element())
            .filter(|e| e.name() == QName::new("WINDOWS"))
            .nth(0)
            .expect("<IMAGE> element without <WINDOWS> child element")
            .children().into_iter()
            .filter_map(|n| n.element())
            .filter(|e| e.name() == QName::new("EDITIONID"))
            .nth(0)
            .expect("<WINDOWS> element without <EDITIONID> child element")
            .children().into_iter()
            .filter_map(|n| n.text())
            .map(|t| t.text())
            .collect();
        index_edition.push((image_index_nz, edition_id));
    }

    // editions with most features per version:
    // Vista, 7: "Ultimate"
    // 8, 10, 11: "Professional" (but this has fewer features than "Ultimate" on Vista and 7)
    // => "Ultimate", then "Professional"

    let ultimate_index = index_edition
        .iter()
        .filter(|(_idx, ed)| ed == "Ultimate")
        .map(|(idx, _ed)| *idx)
        .nth(0);
    let ent_index = index_edition
        .iter()
        .filter(|(_idx, ed)| ed == "Enterprise")
        .map(|(idx, _ed)| *idx)
        .nth(0);
    let pro_index = index_edition
        .iter()
        .filter(|(_idx, ed)| ed == "Professional")
        .map(|(idx, _ed)| *idx)
        .nth(0);
    let best_index = ultimate_index
        .or(ent_index)
        .or(pro_index)
        .expect("found neither Ultimate nor Enterprise nor Professional edition");

    // select that image
    let best_image = wim.select_image(best_index);

    // pick out the paths we want
    let mut want_paths = Vec::new();
    best_image.iterate_dir_tree(
        TStr::from_impl(c"/"),
        IterateDirTreeFlags::RECURSIVE,
        |entry| {
            let want =
                entry.full_path.to_str().to_lowercase() == "/windows/system32"
                || entry.full_path.to_str().to_lowercase() == "/windows/syswow64"
            ;
            if want {
                want_paths.push(CString::new(entry.full_path.to_str().as_bytes()).unwrap());
            }
            Ok(())
        },
    )
        .expect("directory iteration failed");

    let want_paths_tstr: Vec<ThinTStr> = want_paths.iter()
        .map(|p| ThinTStr::new(TStr::from_impl(p)))
        .collect();
    best_image.extract_from_paths(
        &want_paths_tstr,
        TStr::from_impl(&CString::new(opts.out_path.as_os_str().as_encoded_bytes()).expect("output path has NULs")),
        ExtractFlags::empty(),
    )
        .expect("extraction failed");
}
