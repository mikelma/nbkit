use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum NbpmError {
    ConfigLoad(Box<dyn Error>),
}

impl fmt::Display for NbpmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            NbpmError::ConfigLoad(e) => write!(f, "Cannot load configuration: {}", e),
        }
    }
}

impl Error for NbpmError {}
