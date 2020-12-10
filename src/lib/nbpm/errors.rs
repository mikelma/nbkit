use std::error::Error;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum NbpmError {
    ConfigLoad(Box<dyn Error>),
    LocalDbLoad(String),
    RepoIndexLoad(String),
    CleanUnSuccessfulInstallation,
    /// Contains paths to the files that couldn't be removed  
    DirtyUnSuccessfulInstallation(Vec<PathBuf>),
}

impl fmt::Display for NbpmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            NbpmError::ConfigLoad(e) => write!(f, "Cannot load configuration: {}", e),
            NbpmError::LocalDbLoad(e) => write!(f, "Cannot load local package database: {}", e),
            NbpmError::RepoIndexLoad(e) => write!(f, "Cannot load repository index: {}", e),
            NbpmError::CleanUnSuccessfulInstallation => write!(
                f,
                "Clean unsuccessful installation. All installed files has been removed."
            ),
            NbpmError::DirtyUnSuccessfulInstallation(paths) => {
                write!(
                    f,
                    "Dirty unsuccessful installation. Cannot remove some instaled files:",
                )?;
                for p in paths {
                    writeln!(f, "  {}", p.display())?;
                }
                Ok(())
            }
        }
    }
}

impl Error for NbpmError {}
