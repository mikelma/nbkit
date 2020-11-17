//! In this module contains all defenitions for nebula repostories.
//!
//! **NOTE**: All paths defined below are relative to the root of the repository,
//! the repo/{architecture} directory.

/// Filename of the index `PkgDb`.
pub const REPO_INDEX_PATH: &'static str = "index.toml";

/// Path to the directory where binary packages are.
pub const REPO_BIN_DIR: &'static str = "bin";

/// Path to the directory where source file of the packages are.
pub const REPO_SRC_DIR: &'static str = "src";
