use semver::{Version, VersionReq};

use std::error::Error;
use std::fmt;

use super::Set;

#[derive(Debug)]
pub enum NbError {
    // ----------- IO ----------- //
    /// Contains the name of the missing file.
    MissingFile(String),
    // ----- Package related ---- //
    /// Contains the name of the missing dependecy and the name of package that requires the
    /// dependecy.
    MissingDependency(String, String),
    /// Contains the name of the broken dependecy, the expected version, the actual verison of the
    /// dependecy and the name of the package that requires the dependecy.
    BrokenDependency(String, VersionReq, Version, String),
    /// Contains the name of the package that was not found
    PkgNotFound(String),
    /// Contains the name package of the package that breaks the set consistency and the expected
    /// set.
    BrokenSetConsistency(String, Set),
    // ------ PkgDb related ---- //
    PkgDbLoad(Box<dyn Error>),
}

impl fmt::Display for NbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            // ----------- IO ----------- //
            NbError::MissingFile(file) => write!(f, "Missing file {}", file),
            // ----- Package related ---- //
            NbError::MissingDependency(dep_name, pkg_name) => {
                write!(f, "Missing dependecy {} required by {}", dep_name, pkg_name)
            }
            NbError::BrokenDependency(dep_name, req, ver, pkg_name) => write!(
                f,
                "Broken dependency. Expected version ({}), got ({}): {} required by {}",
                req.to_string(),
                ver.to_string(),
                dep_name,
                pkg_name,
            ),
            NbError::PkgNotFound(name) => write!(f, "Package {} not found", name),
            NbError::BrokenSetConsistency(name, set) => write!(
                f,
                "Package {} breaks set consistency. The expected set is {}.",
                name, set
            ),
            // ------ PkgDb related ---- //
            NbError::PkgDbLoad(err) => write!(f, "Cannot load PkgDb: {}", err),
        }
    }
}

impl Error for NbError {}
