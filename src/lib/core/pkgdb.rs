use semver::{Version, VersionReq};
use serde_derive::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

use super::{wrappers::*, NbError, Set};
use crate::{TypeErr, DEFAULT_SET};

/// Struct that contains all info about a package from a `PkgDb`.
#[derive(Deserialize, Serialize, Debug)]
pub struct PkgInfo {
    /// The package version must be formatted in semver.
    version: VersionWrap,
    /// Package's depency list.
    depends: Option<Vec<DependencyWrap>>,
    /// Brief description of the package.
    description: String,
    /// Set specific information. It is optional, as meta-packages
    /// have no set info.
    #[serde(flatten)]
    set_info: Option<SetInfo>,
}

impl fmt::Display for PkgInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.version.inner().to_string())?;
        write!(f, "   {}", self.description)
    }
}

impl PkgInfo {
    pub fn from(
        version: VersionWrap,
        depends: Option<Vec<DependencyWrap>>,
        description: String,
        set_info: Option<SetInfo>,
    ) -> PkgInfo {
        PkgInfo {
            version,
            depends,
            description,
            set_info,
        }
    }

    pub fn version(&self) -> &Version {
        self.version.inner()
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn depends(&self) -> Option<Vec<(String, VersionReq)>> {
        match &self.depends {
            Some(list) => Some(
                list.iter()
                    .map(|x| {
                        let a = x.inner();
                        (a.0.clone(), a.1.clone())
                    })
                    .collect(),
            ),
            None => None,
        }
    }

    pub fn get_set_info(&self) -> &Option<SetInfo> {
        &self.set_info
    }
}

/// This enum is used to contain the information struct
/// of the set the package is from.
#[derive(Deserialize, Serialize, Debug)]
pub enum SetInfo {
    #[serde(rename = "universe")]
    Universe(InfoUniverse),
    #[serde(rename = "local")]
    Local(InfoLocal),
}

/// Information about universe's packages.
#[derive(Deserialize, Serialize, Debug)]
pub struct InfoUniverse {
    /// Source to download the package from.
    location: String,
}

impl InfoUniverse {
    pub fn location(&self) -> &str {
        self.location.as_str()
    }
}

/// Information about local packages.
#[derive(Deserialize, Serialize, Debug)]
pub struct InfoLocal {
    paths: Vec<String>,
}

impl InfoLocal {
    pub fn from(paths: Vec<String>) -> InfoLocal {
        InfoLocal { paths }
    }
}

/// Struct to contain the package data base. A package data base contains
/// all information the packages from a `set`. There is some common information
/// packages from any set should have (for example, version and dependencies), and some specific
/// information that depends on the set the package is located.
#[derive(Deserialize, Serialize, Debug)]
pub struct PkgDb {
    /// Set where the packages of the `PkgDb` are located.
    set: Set,
    /// Contains the name and `PkgInfo` of a all packages in the `PkgDb`.
    #[serde(flatten)]
    pkgdata: HashMap<String, PkgInfo>,
}

impl PkgDb {
    /// Creates a new (empty) `PkgDb`. As default, the `set` is `Universe`.
    pub fn new() -> PkgDb {
        PkgDb {
            set: DEFAULT_SET,
            pkgdata: HashMap::new(),
        }
    }

    /// Loads a `PkgDb` from the given `toml` file.
    pub fn load(path: &Path) -> Result<PkgDb, TypeErr> {
        let file_str = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => return Err(Box::new(NbError::PkgDbLoad(Box::new(e)))),
        };
        match toml::from_str::<PkgDb>(&file_str) {
            Ok(db) => Ok(db),
            Err(e) => return Err(Box::new(NbError::PkgDbLoad(Box::new(e)))),
        }
    }

    /// Given a a package name, returns the `PkgInfo` of the package. If the package does not exist
    /// in the `PkgDb`, an error is returned.
    pub fn get_pkg_info(&self, name: &str) -> Result<&PkgInfo, TypeErr> {
        // find the package in the `PkgDb`, if it exists get all info about it
        match self.pkgdata.get(name) {
            Some(v) => Ok(v),
            None => return Err(Box::new(NbError::PkgNotFound(name.to_string()))),
        }
    }

    pub fn get_subgraph(
        &self,
        select: Option<Vec<&str>>,
    ) -> Result<HashMap<String, &PkgInfo>, TypeErr> {
        // packages pending to be processed
        let mut pending: Vec<String> = match select {
            Some(sels) => sels.iter().map(|s| s.to_string()).collect(),
            None => self.pkgdata.keys().cloned().collect(),
        };

        let mut resolved: HashMap<String, &PkgInfo> = HashMap::new();

        while !pending.is_empty() {
            let current = match pending.pop() {
                Some(p) => p,
                // there is no package to process
                None => break,
            };

            // find the package currently being processed
            let pkg = self.get_pkg_info(&current)?;
            if let Some(dependencies) = pkg.depends() {
                for (name, _) in dependencies {
                    if !pending.contains(&name) && !resolved.contains_key(&name) {
                        pending.push(name.clone());
                    }
                }
            }
            resolved.insert(current, pkg);
        }
        // println!("[TODO] broken dependency check as in struct Graph");
        Self::check_subgraph_integrity(&resolved)?;
        Ok(resolved)
    }

    /// This function checks if the integrity of the graph is correct. The integrity is correct
    /// when every dependency of every the node is inside the graph, and the dependencies met the
    /// version requirements the packages have.
    ///
    /// **Note**: The cost of this function is O(n^2).
    //NOTE: Parallelize?
    pub fn check_subgraph_integrity(subgraph: &HashMap<String, &PkgInfo>) -> Result<(), TypeErr> {
        // for every node (package) in the graph
        for (node_name, node) in subgraph.iter() {
            // for each dependency (if some) of the package
            if let Some(dependencies) = node.depends() {
                for (dep_name, version_req) in dependencies {
                    // check if the dependency is in the graph
                    match subgraph.get(&dep_name) {
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

impl Default for PkgDb {
    fn default() -> Self {
        Self::new()
    }
}
