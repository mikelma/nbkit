use semver::Version;

use std::path::PathBuf;

use crate::Dependencies;

/// Container for the set information of the package,
/// as multiple sets exist and each set has different
/// information.
#[derive(Debug)]
pub enum SetInfo {
    /// Packages not installed in the system, but available to install.
    Universe(InfoUniverse),
    /// Packages already installed in the system.
    Local(InfoLocal),
}

/// Information of packages from set `Universe`
#[derive(Debug)]
pub struct InfoUniverse {
    /// URL of the package source.
    source: String,
}

/// Information of packages from set `Universe`
#[derive(Debug)]
pub struct InfoLocal {
    /// Paths on the system belonging to the package.
    paths: Vec<PathBuf>,
}

/// The package abstraction in the nebula packaget manager.
/// Contains all the information about the package,
/// such as name, version dependencies... and its set information too (if the `Package` is not a
/// meta-package).
#[derive(Debug)]
pub struct Package {
    name: String,
    version: Version,
    depends: Dependencies,
    set_info: Option<SetInfo>,
}

impl Package {
    pub fn new_local(
        name: &str,
        version: Version,
        depends: Dependencies,
        paths: Vec<PathBuf>,
    ) -> Package {
        Package {
            name: name.to_string(),
            version,
            depends,
            set_info: Some(SetInfo::Local(InfoLocal { paths })),
        }
    }

    pub fn new_universe(
        name: &str,
        version: Version,
        depends: Dependencies,
        source: &str,
    ) -> Package {
        Package {
            name: name.to_string(),
            version,
            depends,
            set_info: Some(SetInfo::Universe(InfoUniverse {
                source: source.to_string(),
            })),
        }
    }

    pub fn new_meta(name: &str, version: Version, depends: Dependencies) -> Package {
        Package {
            name: name.to_string(),
            version,
            depends,
            set_info: None,
        }
    }

    /// Returns `true` if the `Package` is a meta-package.
    pub fn is_meta(&self) -> bool {
        self.set_info.is_none()
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn depends(&self) -> &Dependencies {
        &self.depends
    }
}
