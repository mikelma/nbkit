use std::fs;
use std::path::Path;

use super::NbpmError;
use crate::core::{pkgdb::PkgInfo, PkgDb, SetInfo};
use crate::TypeErr;

pub fn remove_handler(
    to_remove: &[&str],
    recursive: bool,
    ask_user: bool,
    check_conflicts: bool,
    local_db: &mut PkgDb,
) -> Result<(), TypeErr> {
    let graph = local_db.get_subgraph(Some(&to_remove), recursive)?;

    if ask_user {
        // ask the user for confirmation before removing the packages
        println!(
            "The following packages are going to be removed ({}):",
            graph.len()
        );
        graph
            .iter()
            .for_each(|(name, info)| println!("     {} {}", name, info.version()));
        match crate::utils::read_line("\nAre you sure you want to remove this packages? [Y/n] ") {
            Ok(line) => {
                if !line.is_empty() && line != "y" && line != "Y" {
                    println!("Operation cancelled");
                    return Ok(());
                }
            }
            Err(e) => return Err(e),
        }
    }

    if check_conflicts {
        println!("[*] Checking for conflicts...");
        let to_remove_names: Vec<&str> = graph.keys().map(|k| k.as_str()).collect();
        local_db.check_remove(to_remove_names)?;
    }

    let mut errors = vec![];
    for (pkg_name, pkg_info) in graph {
        println!("Removing {}...", pkg_name);
        // remove package's files
        if let Err(err) = remove_local_pkg_files(pkg_info) {
            eprintln!("Error while removing {}\n", pkg_name);
            errors.push((pkg_name.to_string(), err));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(Box::new(NbpmError::CannotRemovePkgs(errors)))
    }
}

/// Given a reference of a package `PkgInfo`, the function removes the locally installed package
/// files listed in the `PkgInfo`.
///
/// # Errors
///
/// If errors occur during the removal process, the path to the file/dir that cannot be removed and
/// the resulting error are returned inside a `CannotRemove` error.
pub fn remove_local_pkg_files(info: &PkgInfo) -> Result<(), TypeErr> {
    let paths = match info.set_info() {
        Some(set) => match set {
            SetInfo::Local(l) => l.paths(),
            SetInfo::Universe(_) => unimplemented!(),
        },
        None => return Ok(()), // the package is a metapackage
    };

    let mut errors = vec![];
    let mut dirs = vec![];
    // in this loop, only files are deleted, directories are ignored
    paths.iter().map(|p| Path::new(p)).for_each(|p| {
        if p.is_dir() {
            dirs.push(p);
        } else if let Err(e) = remove_path(p) {
            errors.push((p.to_path_buf(), e));
        }
    });

    // now that all files are removed, try to remove directories. As directories only get removed
    // if they are empty
    dirs.iter().for_each(|p| {
        if let Err(e) = remove_path(p) {
            errors.push((p.to_path_buf(), e));
        }
    });
    if errors.is_empty() {
        Ok(())
    } else {
        Err(Box::new(NbpmError::CannotRemove(errors)))
    }
}

pub fn remove_path(path: &Path) -> Result<(), TypeErr> {
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
