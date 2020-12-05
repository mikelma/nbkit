pub mod cli;
pub mod config;
pub mod errors;

pub use config::Config;
pub use errors::NbpmError;

use std::fs;
use std::path::Path;

use crate::TypeErr;

// constant and default variables of nbpm

/// This is the default directory for nbpm. Here, all configuration and other nbpm specific files
/// are stored.
pub const DEF_NBPM_PATH: &'static str = "/etc/nbpm";

/// The default URL to a nebula repository.
pub const DEF_REPO: &'static str = "www.nebula.com/repo/x86_64";

// NOTE: All paths below are relative paths to nbpm's root directory (default : `DEF_NBPM_PATH`)

/// Name for the nbpm configuration file.
pub const NBPM_CONFIG_FILE: &'static str = "config.toml";

/// File name for the local `PkgDb`. In this file all inforamtion relative to the installed
/// packages is stored.
pub const LOCAL_DB_PATH: &'static str = "local_db.toml";

/// Path where the repository index is stored.
pub const LOCAL_INDEX_PATH: &'static str = "index/index.toml";

/// Path to the working directory of nbpm. The packages being installed will be downloaded in this
/// path.
pub const NBPM_WORK_DIR: &'static str = "/tmp/nbpm";

/// Creates the working directory of nbpm according to `nbpm::NBPM_WORK_DIR`. If the directory
/// already exits, this function does nothing.
pub fn init_working_dir() -> Result<(), TypeErr> {
    // check if the working dir already exists
    if !Path::new(NBPM_WORK_DIR).is_dir() {
        // create the working dir
        if let Err(e) = fs::create_dir(NBPM_WORK_DIR) {
            return Err(Box::new(e));
        }
    }
    Ok(())
}
