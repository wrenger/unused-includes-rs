use std::path::{Path, PathBuf};

use multimap::MultiMap;

use super::fileio;
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
                    if util::is_header_file(&path) {
                        add_file(&path, &mut map, directories);
                    }
                }
            }
        }
    }

    map
}

pub fn print_dependency_tree<P: AsRef<Path>>(
    root: P,
    index: &MultiMap<PathBuf, PathBuf>,
    indent: usize,
) {
    if indent > 0 {
        for _ in 1..indent {
            print!("    ");
        }
        print!("  - ");
    }
    println!("{}", root.as_ref().to_string_lossy());
    if let Some(children) = index.get_vec(root.as_ref()) {
        for child in children {
            print_dependency_tree(child, index, indent + 1);
        }
    }
}

fn add_file<P: AsRef<Path>>(
    file: P,
    map: &mut MultiMap<PathBuf, PathBuf>,
    include_paths: &[PathBuf],
) {
    for include in fileio::parse_includes(&file) {
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
