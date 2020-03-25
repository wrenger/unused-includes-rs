use std::collections::HashSet;
use std::path::{Path, PathBuf};

use multimap::MultiMap;

use super::fileio;
use super::util;

/// Creates an index with all sources and their dependencies (sources that include them).
pub fn index<P>(
    files: &[P],
    directories: &[PathBuf],
    filter: &regex::Regex,
) -> MultiMap<PathBuf, PathBuf>
where
    P: AsRef<Path>,
{
    let mut map = MultiMap::new();

    for file in files {
        if filter.is_match(file.as_ref().to_str().unwrap()) {
            add_file(file.as_ref(), &mut map, directories);
        }
    }

    for dir in directories {
        if let Ok(read_dir) = util::read_dir_rec(dir) {
            for path in read_dir {
                if let Ok(path) = path {
                    let path = path.path();
                    if filter.is_match(path.to_str().unwrap()) && util::is_header_file(&path) {
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
    let mut visited = HashSet::new();
    print_dependency_tree_impl(root, index, indent, &mut visited);
}

fn print_dependency_tree_impl<P: AsRef<Path>>(
    root: P,
    index: &MultiMap<PathBuf, PathBuf>,
    indent: usize,
    visited: &mut HashSet<PathBuf>,
) {
    if indent > 0 {
        for _ in 1..indent {
            print!("    ");
        }
        print!("  - ");
    }
    if visited.insert(PathBuf::from(root.as_ref())) {
        println!("{}", root.as_ref().to_string_lossy());

        if let Some(children) = index.get_vec(root.as_ref()) {
            for child in children {
                print_dependency_tree_impl(child, index, indent + 1, visited);
            }
        }
    } else {
        println!("!circular: {}", root.as_ref().to_string_lossy());
    }
}

fn add_file<P: AsRef<Path>>(
    file: P,
    map: &mut MultiMap<PathBuf, PathBuf>,
    include_paths: &[PathBuf],
) {
    for include in fileio::parse_includes(&file) {
        if let Some(include) = util::find_include(&file, &include, include_paths) {
            if let Ok(include) = include.canonicalize() {
                if let Ok(file) = file.as_ref().canonicalize() {
                    map.insert(include, file);
                }
            }
        } else {
            println!("Missing include {} {:?}", include, file.as_ref());
        }
    }
}
