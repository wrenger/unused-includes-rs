use std::collections::HashSet;
use std::path::{Path, PathBuf};

use multimap::MultiMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::fileio;
use super::util;

pub struct Dependencies {
    index: MultiMap<PathBuf, PathBuf>,
}

impl Dependencies {
    pub fn new() -> Dependencies {
        Dependencies {
            index: MultiMap::new(),
        }
    }

    /// Creates an index with all sources and their dependencies (sources that include them).
    pub fn create(files: &[&Path], directories: &[PathBuf], filter: &regex::Regex) -> Dependencies {
        let mut dependencies = Dependencies {
            index: MultiMap::new(),
        };

        for file in files {
            if filter.is_match(file.to_str().unwrap()) {
                dependencies.add(file.as_ref(), directories);
            }
        }

        for dir in directories {
            if let Ok(read_dir) = util::read_dir_rec(dir) {
                for path in read_dir {
                    let path = path.unwrap().path();
                    if filter.is_match(path.to_str().unwrap()) && util::is_header_file(&path) {
                        dependencies.add(&path, directories);
                    }
                }
            }
        }

        dependencies
    }

    pub fn add(&mut self, file: &Path, include_paths: &[PathBuf]) {
        for include in fileio::parse_includes(file) {
            if let Some(include) = util::find_include(file, Path::new(&include), include_paths) {
                if let Ok(include) = include.canonicalize() {
                    if let Ok(file) = file.canonicalize() {
                        self.index.insert(include, file);
                    }
                }
            } else {
                eprintln!("Missing include {} {:?}", include, file);
            }
        }
    }

    pub fn get(&self, file: &Path) -> &[PathBuf] {
        if let Some(result) = self.index.get_vec(file) {
            result
        } else {
            &[]
        }
    }

    /// Print the dependency tree with the given `root` file
    pub fn print(&self, root: &Path) {
        let mut visited = HashSet::new();
        self.print_impl(root.as_ref(), 0, &mut visited);
    }

    fn print_impl<'a>(&'a self, root: &'a Path, indent: usize, visited: &mut HashSet<&'a Path>) {
        if indent > 0 {
            for _ in 1..indent {
                print!("    ");
            }
            print!("  - ");
        }
        if visited.insert(root) {
            println!("{}", root.to_string_lossy());
            for child in self.get(root) {
                self.print_impl(child, indent + 1, visited);
            }
        } else {
            println!("!circular: {}", root.to_string_lossy());
        }
    }
}

impl Serialize for Dependencies {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.index.serialize(serializer)
    }
}

impl<'a> Deserialize<'a> for Dependencies {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        Ok(Dependencies {
            index: MultiMap::deserialize(deserializer)?,
        })
    }
}
