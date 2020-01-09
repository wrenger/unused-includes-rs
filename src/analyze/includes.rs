use std::collections::{HashMap, HashSet};
use std::iter::{self, FromIterator};
use std::usize;

pub type FileID = (u64, u64, u64);

#[derive(Debug)]
struct IncludeEntry {
    includes: HashSet<FileID>,
    used: bool,
    costs: usize,
    pred: Option<FileID>,
}

impl IncludeEntry {
    fn new() -> IncludeEntry {
        IncludeEntry {
            includes: HashSet::new(),
            used: false,
            costs: 0,
            pred: None,
        }
    }

    fn new_with(include: FileID) -> IncludeEntry {
        IncludeEntry {
            includes: HashSet::from_iter(iter::once(include)),
            used: false,
            costs: 0,
            pred: None,
        }
    }
}

#[derive(Debug)]
pub struct IncludeGraph {
    includes: HashMap<FileID, IncludeEntry>,
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

    pub fn insert(&mut self, from: FileID, to: FileID) {
        if let Some(entry) = self.includes.get_mut(&from) {
            entry.includes.insert(to);
        } else {
            self.includes.insert(from, IncludeEntry::new_with(to));
        }
    }

    pub fn mark_used(&mut self, key: &FileID) {
        if let Some(entry) = self.includes.get_mut(key) {
            entry.used = true;
        } else {
            let mut entry = IncludeEntry::new();
            entry.used = true;
            self.includes.insert(key.clone(), entry);
        }
    }

    pub fn unused(&mut self, main: &FileID) -> HashSet<FileID> {
        self.shortest_paths(main);

        let mut result = HashSet::new();

        if let Some(main) = self.includes.get(main) {
            result.extend(main.includes.iter());
        }

        for val in self.includes.values() {
            if val.used {
                if let Some(pred) = &val.pred {
                    result.remove(pred);
                }
            }
        }

        result
    }

    /// Inspired by the Bellman-Ford-Moore algorithm
    fn shortest_paths(&mut self, start: &FileID) {
        for node in self.includes.values_mut() {
            node.costs = usize::MAX - 1;
        }

        if let Some(s_node) = self.includes.get_mut(start) {
            s_node.costs = 0;
            s_node.pred = Some(start.clone());
        }

        let edges = self.edges();

        // Setup direct successors (set predecessors to themselves)
        for direct_successor in edges.iter().filter(|e| &e.0 == start) {
            if let Some(node) = self.includes.get_mut(&direct_successor.1) {
                node.costs = 1;
                node.pred = Some((&direct_successor.1).clone());
            }
        }

        for _ in 1..self.len() {
            for (u, v) in &edges {
                if let Some(u_node) = self.includes.get(u) {
                    let (u_costs, u_pred) = (u_node.costs, u_node.pred.clone());

                    if let Some(v_node) = self.includes.get_mut(v) {
                        if u_costs + 1 < v_node.costs {
                            v_node.costs = u_costs + 1;
                            v_node.pred = u_pred;
                        }
                    }
                }
            }
        }
    }

    fn edges(&self) -> Vec<(FileID, FileID)> {
        let mut edges = Vec::with_capacity(self.len());
        for (key, value) in &self.includes {
            for to in &value.includes {
                edges.push((key.clone(), to.clone()));
            }
        }
        edges
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_use_shortest_paths() {
        let mut graph = IncludeGraph::new();
        graph.insert((0, 0, 0), (1, 0, 0));
        graph.insert((1, 0, 0), (2, 0, 0));
        graph.insert((2, 0, 0), (3, 0, 0)); // used, costs 3

        graph.insert((0, 0, 0), (0, 1, 0)); // used

        graph.insert((0, 0, 0), (0, 0, 1));
        graph.insert((0, 0, 1), (0, 0, 2));
        graph.insert((0, 0, 1), (3, 0, 0)); // used, costs 2

        graph.mark_used(&(0, 1, 0));
        graph.mark_used(&(3, 0, 0));

        println!("graph {:?}", graph);

        graph.shortest_paths(&(0, 0, 0));

        println!("short graph:");
        for (key, val) in &graph.includes {
            println!("- {:?}: {:?}", key, val);
        }

        assert_eq!(
            &HashSet::from_iter(iter::once((1, 0, 0))),
            &graph.unused(&(0, 0, 0))
        );
    }
}
