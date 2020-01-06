use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use multimap::MultiMap;

pub fn index(filter: &str) -> MultiMap<String, PathBuf> {
    let mut map = MultiMap::new();

    for path in glob::glob(filter)
        .expect("Malformed filter pattern")
        .filter_map(Result::ok)
    {
        if is_sourcefile(&path) {
            println!("Deps match {:?}", path);
            for include in parse_includes(&path) {
                map.insert(include, path.clone());
            }
        }
    }

    map
}

fn is_sourcefile<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    if path.is_file() {
        if let Some(extension) = path.extension() {
            extension == "c" || extension == "h" || extension == "cpp" || extension == "hpp"
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
