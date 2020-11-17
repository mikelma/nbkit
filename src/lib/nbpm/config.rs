use serde_derive::{Deserialize, Serialize};

use std::fs::read_to_string;
use std::path::Path;

use super::{NbpmError, DEF_NBPM_PATH, DEF_REPO};
use crate::TypeErr;

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    #[serde(rename = "nbpm-home", default = "get_default_nbpm_home")]
    home: String,
    repo_url: String,
}

impl Config {
    /// Creates the default `Config` object.
    pub fn new() -> Config {
        Config {
            home: DEF_NBPM_PATH.to_string(),
            repo_url: DEF_REPO.to_string(),
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
            Err(e) => return Err(Box::new(NbpmError::ConfigLoad(Box::new(e)))),
        }
    }

    pub fn home(&self) -> &str {
        &self.home
    }

    pub fn repo_url(&self) -> &str {
        &self.repo_url
    }
}

fn get_default_nbpm_home() -> String {
    DEF_NBPM_PATH.to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
