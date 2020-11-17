use std::collections::HashMap;
use std::fmt;

use super::{NbError, Package, Set};
use crate::{TypeErr, DEFAULT_SET};

#[derive(Debug)]
pub struct Graph {
    set: Set,
    map: HashMap<String, Package>,
}

impl Graph {
    pub fn new() -> Graph {
        Graph {
            map: HashMap::new(),
            set: DEFAULT_SET,
        }
    }

    /// Creates a new `Graph` from the given `Set` and map.
    /// If `check` is `true`, the function internally calls `check_integrity` function to ensure
    /// that the integrity of the generated `Graph` is correct. Set `check` to `false` when the
    /// integrity of the generated graph is ensured to be correct.
    pub fn from(set: Set, map: HashMap<String, Package>, check: bool) -> Result<Graph, TypeErr> {
        let g = Graph { set, map };
        if check {
            if let Err(e) = g.check_integrity() {
                return Err(e);
            }
        }
        Ok(g)
    }

    pub fn contains_node(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }

    pub fn add_node(&mut self, node: Package) {
        if !self.contains_node(node.name()) {
            self.map.insert(node.name().to_string(), node);
        }
    }

    /// This function checks if the integrity of the graph is correct. The integrity is correct
    /// when every dependency of every the node is inside the graph, and the dependencies met the
    /// version requirements the packages have.
    ///
    /// **Note**: The cost of this function is O(n^2).
    //NOTE: Parallelize?
    pub fn check_integrity(&self) -> Result<(), TypeErr> {
        // for every node (package) in the graph
        for (node_name, node) in self.map.iter() {
            // for each dependency (if some) of the package
            if let Some(dependencies) = node.depends() {
                for (dep_name, version_req) in dependencies {
                    // check if the dependency is in the graph
                    match self.map.get(dep_name) {
                        // if the dependency exists, check if the version requirement is met
                        Some(dep) => {
                            if !version_req.matches(dep.version()) {
                                return Err(Box::new(NbError::BrokenDependency(
                                    dep_name.to_string(),
                                    version_req.clone(),
                                    dep.version().clone(),
                                    node_name.to_string(),
                                )));
                            }
                        }
                        // the dependency is missing in the graph
                        None => {
                            return Err(Box::new(NbError::MissingDependency(
                                dep_name.to_string(),
                                node_name.to_string(),
                            )))
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl fmt::Display for Graph {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "* Set: {:?}", self.set)?;
        writeln!(f, "* Packages:")?;
        for node in self.map.values() {
            writeln!(f, "    - {} {}", node.name(), node.version().to_string())?;
            if let Some(deps) = node.depends() {
                write!(f, "        depends:")?;
                for (dname, vreq) in deps {
                    write!(f, " [{}{}] ", dname, vreq)?;
                }
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}
