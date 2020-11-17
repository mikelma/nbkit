use serde_derive::{Deserialize, Serialize};

use std::fmt;

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone, Copy)]
pub enum Set {
    #[serde(rename = "universe")]
    Universe,
    #[serde(rename = "local")]
    Local,
}

impl fmt::Display for Set {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Set::Universe => write!(f, "universe"),
            Set::Local => write!(f, "local"),
        }
    }
}
