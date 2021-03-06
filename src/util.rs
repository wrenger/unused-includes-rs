use std::fs::{self, DirEntry, ReadDir};
use std::io;
use std::path::{Path, PathBuf};

/// Returns whether the given path points to a header file
pub fn is_header_file(path: &Path) -> bool {
    path.is_file()
        && matches!(path.extension(),
            Some(e) if e == "h" || e == "hpp")
}

/// Parses the include paths from the given compiler commandline.
pub fn include_paths(command: &str) -> impl Iterator<Item = &str> {
    lazy_static::lazy_static! {
        static ref INCLUDE_RE: regex::Regex =
            regex::Regex::new("(^|\\s)-I ?([\\w\\-/\\.]+)").unwrap();
    }

    INCLUDE_RE
        .captures_iter(command)
        .map(|caps| caps.get(2).unwrap().as_str())
}

/// Finds the corresponding filepath to the given `include`.
///
/// It also handles relative includes and 'src/main/...' is correctly resolved.
pub fn find_include(file: &Path, include: &Path, include_paths: &[PathBuf]) -> Option<PathBuf> {
    // Relative to file
    if let Some(parent) = file.parent() {
        let path = parent.join(include);
        if path.exists() {
            return Some(path);
        }
    }

    // Looking for absolute includes in the given include paths
    for include_path in include_paths {
        let path = include_path.join(&include);
        if path.exists() {
            return Some(path);
        }
    }

    // Looking for relative includes
    // Also 'src/main/...' is correctly resolved
    let mut relpath: PathBuf = file
        .components()
        .map(|e| e.as_os_str())
        .skip_while(|&e| e != "include" && e != "src")
        .skip(1)
        .skip_while(|&e| e == "main")
        .collect();

    if !relpath.as_os_str().is_empty() {
        relpath.pop(); // Remove filename
        for include_path in include_paths {
            let path = include_path.join(&relpath).join(&include);
            if path.exists() {
                return Some(path);
            }
        }
    }

    None
}

/// Walks through the given directory tree recursively.
pub fn read_dir_rec(path: &Path) -> io::Result<ReadDirRec> {
    Ok(ReadDirRec {
        dirs: vec![fs::read_dir(path)?],
    })
}

/// Iterator for recursive directory traversal
pub struct ReadDirRec {
    dirs: Vec<ReadDir>,
}

impl Iterator for ReadDirRec {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(dir) = self.dirs.last_mut() {
            match dir.next() {
                Some(Ok(entry)) => {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            if let Ok(read_dir) = fs::read_dir(entry.path()) {
                                self.dirs.push(read_dir);
                            }
                        }
                    }
                    Some(Ok(entry))
                }
                Some(err) => Some(err),
                None => {
                    self.dirs.pop();
                    self.next()
                }
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_dir_rec() {
        for path in read_dir_rec(Path::new("target")).unwrap() {
            println!("{:?}", path.unwrap().path());
        }
    }

    #[test]
    fn test_find_include() {
        assert_eq!(
            find_include(
                Path::new("tests/src/refs/Main.cpp"),
                Path::new("Functions.hpp"),
                &[PathBuf::from("tests/src")],
            ),
            Some(PathBuf::from("tests/src/refs/Functions.hpp"))
        );
        assert_eq!(
            find_include(
                Path::new("tests/src/refs/Main.cpp"),
                Path::new("Unused.hpp"),
                &[PathBuf::from("tests/src")],
            ),
            Some(PathBuf::from("tests/src/Unused.hpp"))
        );
    }
}
