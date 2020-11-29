use semver::{Version, VersionReq};
use serde_derive::{Deserialize, Serialize};

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::{wrappers::*, Graph, NbError, Package, Set};
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

impl PkgInfo {
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
}

/// This enum is used to contain the information struct
/// of the set the package is from.
#[derive(Deserialize, Serialize, Debug)]
enum SetInfo {
    #[serde(rename = "universe")]
    Universe(InfoUniverse),
    #[serde(rename = "local")]
    Local(InfoLocal),
}

/// Information about universe's packages.
#[derive(Deserialize, Serialize, Debug)]
struct InfoUniverse {
    /// Source to download the package from.
    pub source: String,
}

/// Information about local packages.
#[derive(Deserialize, Serialize, Debug)]
pub struct InfoLocal {
    paths: Vec<String>,
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

    /// Given a package name, searches for the package. If the it exists in the `PkgDb`, the
    /// function returns the `Package` object of that package.
    /// If the package does not exit in the `PkgDb` the function returns an error.
    ///
    /// **Important note**: Not all the information contained in the `PkgDb` abput a package is
    /// transfered to the actual `Package` object. For example the description of a package is not
    /// included in the `Package` object. This is done for efficiency purposes.
    pub fn get_package(&self, name: &str) -> Result<Package, TypeErr> {
        // find the package in the `PkgDb`, if it exists get all info about it
        let pkg_info = self.get_pkg_info(name)?;
        // get all the info we have about the package

        // let version = Version::parse(&pkg_info.version)?;
        let version = pkg_info.version();

        // format all the dependencies of the package
        /*
        let depends = match &pkg_info.depends {
            Some(deps_list) => {
                let mut queries = vec![];
                for dep_str in deps_list {
                    queries.push(utils::parse_pkg_str_info(&dep_str)?);
                }
                Some(queries)
            }
            None => None,
        };
        */
        let depends = match pkg_info.depends() {
            Some(list) => Some(
                list.iter()
                    .map(|(name, vreq)| (name.to_string(), vreq.clone()))
                    .collect(),
            ),
            None => None,
        };
        // get the set specific information and build the package
        match &pkg_info.set_info {
            Some(SetInfo::Universe(set_info)) => {
                // check if the package is from the same set as the `PkgDb`
                if self.set == Set::Universe {
                    Ok(Package::new_universe(
                        name,
                        version.clone(),
                        depends,
                        &set_info.source,
                    ))
                } else {
                    return Err(Box::new(NbError::BrokenSetConsistency(
                        name.to_string(),
                        self.set,
                    )));
                }
            }
            Some(SetInfo::Local(set_info)) => {
                // check if the package is from the same set as the `PkgDb`
                if self.set == Set::Local {
                    // convert strings to PathBufs
                    let pathbufs = set_info.paths.iter().map(PathBuf::from).collect();
                    Ok(Package::new_local(name, version.clone(), depends, pathbufs))
                } else {
                    return Err(Box::new(NbError::BrokenSetConsistency(
                        name.to_string(),
                        self.set,
                    )));
                }
            }
            None => {
                // the package is a mata-package as metas have no package info
                Ok(Package::new_meta(name, version.clone(), depends))
            }
        }
    }

    pub fn get_graph(&self, select: Option<Vec<&str>>) -> Result<Graph, TypeErr> {
        // packages pending to be processed
        let mut pending: Vec<String> = match select {
            Some(sels) => sels.iter().map(|s| s.to_string()).collect(),
            None => self.pkgdata.keys().cloned().collect(),
        };

        let mut resolved: HashMap<String, Package> = HashMap::new();

        while !pending.is_empty() {
            let current = match pending.pop() {
                Some(p) => p,
                // there is no package to process
                None => break,
            };

            // find the package currently being processed
            let pkg = self.get_package(&current)?;
            if let Some(dependencies) = pkg.depends() {
                for (name, _) in dependencies {
                    if !pending.contains(name) && !resolved.contains_key(name) {
                        pending.push(name.clone());
                    }
                }
            }
            resolved.insert(current, pkg);
        }
        // generate the graph with the resolved packages, note that
        // the integrity check is enabled, as the integrity of the graph is not ensured
        Graph::from(self.set, resolved, true)
    }

    /*
    pub fn get_graph_2(
        &self,
        select: Option<Vec<&str>>,
    ) -> Result<HashMap<String, Rc<PkgInfo>>, TypeErr> {
        // packages pending to be processed
        let mut pending: Vec<String> = match select {
            Some(sels) => sels.iter().map(|s| s.to_string()).collect(),
            None => self.pkgdata.keys().cloned().collect(),
        };

        let mut resolved: HashMap<String, Rc<Package>> = HashMap::new();

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
                    if !pending.contains(name) && !resolved.contains_key(name) {
                        pending.push(name.clone());
                    }
                }
            }
            resolved.insert(current, pkg);
        }
        println!("[TODO] broken dependency check as in struct Graph");
        Ok(resolved)
    }
    */
}

impl Default for PkgDb {
    fn default() -> Self {
        Self::new()
    }
}
