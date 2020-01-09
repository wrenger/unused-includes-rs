use std::fs::{self, DirEntry, ReadDir};
use std::io;
use std::path::{Path, PathBuf};

/// Parses the include paths from the given compiler commandline.
pub fn include_paths<'a>(command: &'a str) -> impl Iterator<Item = &'a str> {
    lazy_static! {
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
pub fn find_include<P, Q>(file: P, include: Q, include_paths: &[PathBuf]) -> Option<PathBuf>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
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
        .as_ref()
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
pub fn read_dir_rec<P: AsRef<Path>>(path: P) -> io::Result<ReadDirRec> {
    Ok(ReadDirRec {
        dirs: vec![fs::read_dir(path)?],
    })
}

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
        for path in read_dir_rec("target").unwrap() {
            println!("{:?}", path.unwrap().path());
        }
    }

    #[test]
    fn test_find_include() {
        assert_eq!(
            find_include(
                "tests/src/refs/Main.cpp",
                "Functions.hpp",
                &[PathBuf::from("tests/src")],
            ),
            Some(PathBuf::from("tests/src/refs/Functions.hpp"))
        );
        assert_eq!(
            find_include(
                "tests/src/refs/Main.cpp",
                "Unused.hpp",
                &[PathBuf::from("tests/src")],
            ),
            Some(PathBuf::from("tests/src/Unused.hpp"))
        );
    }
}
