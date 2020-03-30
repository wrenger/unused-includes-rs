use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::dependencies::Dependencies;
use super::util;

#[derive(Deserialize)]
struct CompilationEntry {
    file: PathBuf,
    command: String,
}

/// Compile commands for the sourcefiles of a project
pub struct Compilations {
    map: HashMap<PathBuf, String>,
}

impl Compilations {
    /// Parse a clang compilation database and collect the compile commands
    pub fn parse<P: AsRef<Path>>(file: P, filter: &regex::Regex) -> Result<Compilations, String> {
        let file = File::open(file).map_err(|e| format!("{}", e))?;
        let commands: Vec<CompilationEntry> =
            serde_yaml::from_reader(file).map_err(|e| format!("{}", e))?;

        Ok(Compilations {
            map: HashMap::from_iter(
                commands
                    .into_iter()
                    .filter(|e| filter.is_match(&e.file.to_str().expect("Malformed db source")))
                    .map(|e| (e.file, e.command)),
            ),
        })
    }

    /// Collect all include paths contained in the compile commands
    pub fn collect_include_paths(&self) -> Vec<PathBuf> {
        let mut paths: HashSet<PathBuf> = HashSet::new();

        for command in self.map.values() {
            for path in util::include_paths(command) {
                if !paths.contains(Path::new(path)) {
                    paths.insert(PathBuf::from(path));
                }
            }
        }

        paths.into_iter().collect()
    }

    /// Retrieve the corresponding compiler arguments
    pub fn get_related_args<P: AsRef<Path>>(
        &self,
        file: P,
        index: &Dependencies,
    ) -> Option<Vec<String>> {
        if let Some(command) = self.map.get(file.as_ref()) {
            parse_args(command)
        } else {
            let dependencies = index.get(file);
            // Check direct dependencies first
            for dependency in dependencies {
                if let Some(command) = self.map.get(dependency) {
                    return parse_args(command);
                }
            }
            // Search whole subtree
            for dependency in dependencies {
                if let Some(args) = self.get_related_args(dependency, index) {
                    return Some(args);
                }
            }

            None
        }
    }

    /// Return all sources
    pub fn sources(&self) -> Vec<&PathBuf> {
        self.map.keys().collect()
    }
}

fn parse_args(command: &str) -> Option<Vec<String>> {
    // Skip compiler
    if let Some(start) = command.find(' ') {
        if let Some(mut args) = shlex::split(&command[start..]) {
            args.pop(); // Remove the input file arg
            if let Some(pos) = args.iter().position(|e| e == "-o") {
                // Remove '-o <outfile>'
                if pos + 1 < args.len() {
                    args.remove(pos + 1);
                }
                args.remove(pos);
            }

            Some(args)
        } else {
            None
        }
    } else {
        None
    }
}
