use std::path::PathBuf;

pub fn include_paths(args: &[String], filter: &glob::Pattern) -> Vec<PathBuf> {
    vec![]
}

pub fn find_include(include: String, include_paths: &[PathBuf]) -> Option<PathBuf> {
    None
}
