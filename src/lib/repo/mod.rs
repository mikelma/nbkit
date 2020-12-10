//! In this module contains all defenitions for nebula repostories.
//!
//! **NOTE**: All paths defined below are relative to the root of the repository,
//! the repo/{architecture} directory.

/// Filename of the index `PkgDb`.
pub const REPO_INDEX_PATH: &str = "index.toml";

/// Path to the directory where binary packages are.
pub const REPO_BIN_DIR: &str = "bin";

/// Path to the directory where source file of the packages are.
pub const REPO_SRC_DIR: &str = "src";

/// File name for the `PkgInfo` of packages.
pub const REPO_PKG_INFO: &str = "nbinfo.toml";
