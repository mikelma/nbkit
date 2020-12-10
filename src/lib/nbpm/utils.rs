use walkdir::WalkDir;

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::config::Config;
use super::NbpmError;
use super::{LOCAL_DB_PATH, LOCAL_INDEX_PATH};
use crate::core::{pkgdb::PkgInfo, PkgDb, Set, SetInfo};
use crate::repo::REPO_PKG_INFO;
use crate::TypeErr;

pub fn load_pkgdb(config: &Config, set: Set) -> Result<PkgDb, NbpmError> {
    let db_path = format!(
        "{}/{}",
        config.home(),
        match set {
            Set::Local => LOCAL_DB_PATH,
            Set::Universe => LOCAL_INDEX_PATH,
        }
    );

    match PkgDb::load(Path::new(&db_path)) {
        Ok(db) => Ok(db),
        Err(e) => match set {
            Set::Local => Err(NbpmError::LocalDbLoad(format!("{}: {}", db_path, e))),
            Set::Universe => Err(NbpmError::RepoIndexLoad(format!("{}: {}", db_path, e))),
        },
    }
}

pub fn remove_local_pkgs(pkgs: &HashMap<String, PkgInfo>, config: &Config) -> Result<(), TypeErr> {
    for (pkg_name, pkg_info) in pkgs {
        println!("[*] Removing {}", pkg_name);
        let paths = match pkg_info.set_info() {
            Some(set) => match set {
                SetInfo::Local(l) => l.paths(),
                SetInfo::Universe(_) => unimplemented!(),
            },
            None => continue, // if the package is a metapackage
        };
        for path in paths {
            let full_path = Path::new(config.root()).join(path);
            remove_path(&full_path)?;
        }
    }
    Ok(())
}

fn remove_path(path: &Path) -> Result<(), TypeErr> {
    if path.is_file() {
        // if the path is a file, remove the file
        if let Err(e) = fs::remove_file(path) {
            return Err(Box::new(e));
        }
    } else if path.is_dir() {
        // if the path is a directory, only remove the directory if the directory
        // is empty
        match fs::read_dir(path) {
            Ok(entries) => {
                if entries.count() == 0 {
                    if let Err(e) = fs::remove_dir(path) {
                        return Err(Box::new(e));
                    }
                }
            }
            Err(e) => return Err(Box::new(e)),
        }
    }
    Ok(())
}

pub fn install_pkg_files(from: &str, to: &str) -> Result<(), TypeErr> {
    let mut installed_files = vec![];
    let mut success = true;
    for entry in WalkDir::new(from) {
        let real_path = match &entry {
            Ok(v) => v.path(),
            Err(e) => {
                eprintln!("Error: {}", e);
                success = false;
                break;
            }
        };
        // do not install the nbinfo.toml file
        if real_path.file_name().unwrap() == REPO_PKG_INFO {
            continue;
        }

        let virt_path = match real_path.strip_prefix(from) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error: {}", e);
                success = false;
                break;
            }
        };

        if virt_path == Path::new(from).join(REPO_PKG_INFO) {
            continue;
        }

        let new_path = Path::new(to).join(virt_path);

        if real_path.is_dir() && !new_path.exists() {
            if let Err(e) = fs::create_dir(&new_path) {
                eprintln!("Error: {}", e);
                success = false;
                break;
            }
        } else if real_path.is_file() {
            if let Err(e) = fs::copy(real_path, &new_path) {
                eprintln!("Error: {}", e);
                success = false;
                break;
            } else {
                installed_files.push(new_path);
            }
        }
    }

    if success {
        return Ok(());
    }

    let mut cannot_remove = vec![];
    for path_str in installed_files {
        if remove_path(Path::new(&path_str)).is_err() {
            cannot_remove.push(path_str);
        }
    }

    if cannot_remove.is_empty() {
        Err(Box::new(NbpmError::CleanUnSuccessfulInstallation))
    } else {
        Err(Box::new(NbpmError::DirtyUnSuccessfulInstallation(
            cannot_remove,
        )))
    }
}
