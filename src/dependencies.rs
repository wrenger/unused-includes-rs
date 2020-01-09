use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use multimap::MultiMap;

use super::util;

/// Creates an index with all sources and their dependencies (sources that include them).
pub fn index<P>(files: &[P], directories: &[PathBuf]) -> MultiMap<PathBuf, PathBuf>
where
    P: AsRef<Path>,
{
    let mut map = MultiMap::new();

    for file in files {
        add_file(file.as_ref(), &mut map, directories);
    }

    for dir in directories {
        if let Ok(read_dir) = util::read_dir_rec(dir) {
            for path in read_dir {
                if let Ok(path) = path {
                    let path = path.path();
                    if is_header_file(&path) {
                        add_file(&path, &mut map, directories);
                    }
                }
            }
        }
    }

    map
}

fn add_file<P: AsRef<Path>>(
    file: P,
    map: &mut MultiMap<PathBuf, PathBuf>,
    include_paths: &[PathBuf],
) {
    for include in parse_includes(&file) {
        if let Some(include) = util::find_include(&file, &include, include_paths) {
            match (
                PathBuf::from(include).canonicalize(),
                PathBuf::from(file.as_ref()).canonicalize(),
            ) {
                (Ok(include), Ok(file)) => map.insert(include, file),
                _ => {}
            }
        } else {
            println!("Missing include {} {:?}", include, file.as_ref());
        }
    }
}

/// Returns whether the given path points to a header file
fn is_header_file<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    if path.is_file() {
        if let Some(extension) = path.extension() {
            extension == "h" || extension == "hpp"
        } else {
            false
        }
    } else {
        false
    }
}

fn parse_includes<P: AsRef<Path>>(path: P) -> Vec<String> {
    // Only local includes
    lazy_static! {
        static ref INCLUDE_RE: regex::Regex =
            regex::Regex::new("^[ \\t]*#[ \\t]*include[ \\t]*\"([\\./\\w-]+)\"").unwrap();
    }

    // TODO: Ignore includes in #if...#endif
    // Also handle header guards and #pragma once

    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        let mut includes = vec![];

        for line in reader.lines() {
            if let Ok(line) = line {
                if INCLUDE_RE.is_match(&line) {
                    let caps = INCLUDE_RE.captures(&line).unwrap();
                    includes.push(String::from(&caps[1]))
                }
            } else {
                return vec![];
            }
        }
        includes
    } else {
        vec![]
    }
}
