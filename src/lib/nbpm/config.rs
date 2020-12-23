use serde_derive::{Deserialize, Serialize};

use std::fs::read_to_string;
use std::path::Path;

use super::{NbpmError, DEF_NBPM_PATH, DEF_NBPM_REPO, DEF_NBPM_ROOT};
use crate::TypeErr;

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    #[serde(rename = "nbpm-home", default = "get_default_nbpm_home")]
    home: String,
    /// Root directory of the system. In most of the cases you want this variable to be `/`.
    #[serde(rename = "root-dir", default = "get_default_nbpm_root")]
    root: String,
    repo_url: String,
}

impl Config {
    /// Creates the default `Config` object with default values from `lib/nbpm/mod.rs`.
    pub fn new() -> Config {
        Config {
            home: DEF_NBPM_PATH.to_string(),
            root: DEF_NBPM_ROOT.to_string(),
            repo_url: DEF_NBPM_REPO.to_string(),
        }
    }

    /// Loads a `Config` from a toml configuration file.
    pub fn from(path: &Path) -> Result<Config, TypeErr> {
        let cfg_str = match read_to_string(path) {
            Ok(s) => s,
            Err(e) => return Err(Box::new(NbpmError::ConfigLoad(Box::new(e)))),
        };

        match toml::from_str::<Config>(&cfg_str) {
            Ok(c) => Ok(c),
            Err(e) => Err(Box::new(NbpmError::ConfigLoad(Box::new(e)))),
        }
    }

    pub fn home(&self) -> &str {
        &self.home
    }

    pub fn root(&self) -> &str {
        &self.root
    }

    pub fn repo_url(&self) -> &str {
        &self.repo_url
    }
}

fn get_default_nbpm_home() -> String {
    DEF_NBPM_PATH.to_string()
}

fn get_default_nbpm_root() -> String {
    DEF_NBPM_ROOT.to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
