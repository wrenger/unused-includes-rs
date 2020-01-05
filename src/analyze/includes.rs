use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Debug)]
pub struct IncludeGraph {
    includes: HashMap<PathBuf, HashSet<PathBuf>>,
}

impl IncludeGraph {
    pub fn new() -> IncludeGraph {
        IncludeGraph {
            includes: HashMap::new(),
        }
    }

    pub fn insert(&mut self, from: PathBuf, to: PathBuf) {
        if let Some(includes) = self.includes.get_mut(&from) {
            includes.insert(to);
        } else {
            let mut includes = HashSet::new();
            includes.insert(to);
            self.includes.insert(from, includes);
        }
    }

    pub fn get_recurse(&self, key: &PathBuf) -> HashSet<PathBuf> {
        if let Some(includes) = self.includes.get(key) {
            let mut result = includes.clone();

            for include in includes {
                result.extend(self.get_recurse(include));
            }

            result
        } else {
            HashSet::new()
        }
    }

    pub fn flatten(mut self, main: &PathBuf) -> DirectIncludeUsages {
        let mut flat_map = HashMap::new();

        if let Some(main) = self.includes.remove(main) {
            for include in main {
                let includes = self.get_recurse(&include);
                flat_map.insert(include, (false, includes));
            }
        }

        DirectIncludeUsages { includes: flat_map }
    }
}

#[derive(Debug)]
pub struct DirectIncludeUsages {
    includes: HashMap<PathBuf, (bool, HashSet<PathBuf>)>,
}

impl DirectIncludeUsages {
    pub fn mark_used(&mut self, to: &PathBuf) {
        if let Some(include) = self.includes.get_mut(to) {
            include.0 = true;
        }

        for include in self.includes.values_mut() {
            if include.1.contains(to) {
                include.0 = true;
                break;
            }
        }
    }

    pub fn unused<'a>(&'a self) -> Vec<&'a PathBuf> {
        self.includes
            .iter()
            .filter_map(|(path, include)| if !include.0 { Some(path) } else { None })
            .collect()
    }
}