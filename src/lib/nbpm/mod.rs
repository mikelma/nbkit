use std::error::Error;
use std::process::exit;

pub mod cli;
pub mod config;
pub mod errors;
pub mod install;
pub mod remove;
pub mod utils;

pub use config::Config;
pub use errors::NbpmError;

// constant and default variables of nbpm

/// Default root directory of the system for nbpm.
pub const DEF_NBPM_ROOT: &str = "/";

/// This is the default directory for nbpm. Here, all configuration and other nbpm specific files
/// are stored.
pub const DEF_NBPM_PATH: &str = "/etc/nbpm";

/// The default URL to a nebula repository.
pub const DEF_NBPM_REPO: &str = "www.nebula.com/repo/x86_64";

// NOTE: All paths below are relative paths to nbpm's root directory (default : `DEF_NBPM_PATH`)

/// Name for the nbpm configuration file.
pub const NBPM_CONFIG_FILE: &str = "config.toml";

/// File name for the local `PkgDb`. In this file all inforamtion relative to the installed
/// packages is stored.
pub const LOCAL_DB_PATH: &str = "local_db.toml";

/// Path where the repository index is stored.
pub const LOCAL_INDEX_PATH: &str = "index/index.toml";

/// Path to the working directory of nbpm. The packages being installed will be downloaded in this
/// path.
pub const NBPM_WORK_DIR: &str = "/tmp/nbpm";

/// A directory inside `NBPM_WORK_DIR` where the packages will be handled individually. For
/// example, packages will be extracted in this directory in the first steps of the installation
/// process.
pub const NBPM_WORK_CURR: &str = "/tmp/nbpm/current";

pub fn exit_with_err(err: Box<dyn Error>) -> ! {
    eprintln!("Error: {}", err);
    exit(1);
}
