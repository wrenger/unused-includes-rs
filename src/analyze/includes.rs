use std::collections::{HashMap, HashSet};
use std::iter::{self, FromIterator};
use std::path::PathBuf;

#[derive(Debug)]
struct IncludeEntry {
    includes: HashSet<PathBuf>,
    used: bool,
    costs: usize,
}

impl IncludeEntry {
    fn new() -> IncludeEntry {
        IncludeEntry {
            includes: HashSet::new(),
            used: false,
            costs: 0,
        }
    }

    fn new_with(include: PathBuf) -> IncludeEntry {
        IncludeEntry {
            includes: HashSet::from_iter(iter::once(include)),
            used: false,
            costs: 0,
        }
    }
}

#[derive(Debug)]
pub struct IncludeGraph {
    includes: HashMap<PathBuf, IncludeEntry>,
}

impl IncludeGraph {
    pub fn new() -> IncludeGraph {
        IncludeGraph {
            includes: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.includes.len()
    }

    pub fn insert(&mut self, from: PathBuf, to: PathBuf) {
        if let Some(entry) = self.includes.get_mut(&from) {
            entry.includes.insert(to);
        } else {
            self.includes.insert(from, IncludeEntry::new_with(to));
        }
    }

    pub fn mark_used(&mut self, key: &PathBuf) {
        if let Some(entry) = self.includes.get_mut(key) {
            entry.used = true;
        } else {
            let mut entry = IncludeEntry::new();
            entry.used = true;
            self.includes.insert(key.clone(), entry);
        }
    }

    pub fn unused<'a>(&'a self, main: &PathBuf) -> HashSet<&'a PathBuf> {
        let mut result = HashSet::new();

        if let Some(entry) = self.includes.get(main) {
            for (i, include) in entry.includes.iter().enumerate() {
                if !self.is_used_impl(include, i + 1) {
                    result.insert(include);
                }
            }
            if !entry.includes.is_empty() {
                for entry in self.includes.values() {
                    unsafe {
                        let costs = &entry.costs as *const usize as *mut usize;
                        *costs = 0; // reset
                    }
                }
            }
        }

        result
    }

    /// Waring: breaks const contract by marking visited nodes
    fn is_used_impl<'a>(&'a self, key: &PathBuf, id: usize) -> bool {
        if let Some(entry) = self.includes.get(key) {
            if entry.costs == id {
                false // Circle detected!
            } else if entry.used {
                true
            } else {
                unsafe {
                    let costs = &entry.costs as *const usize as *mut usize;
                    *costs = id; // Mark as visited
                }
                for include in &entry.includes {
                    if self.is_used_impl(include, id) {
                        return true;
                    }
                }
                false
            }
        } else {
            false
        }
    }
}
