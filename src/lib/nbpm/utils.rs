use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::io::{stdin, stdout, Write};
use std::path::Path;

use super::{config::Config, NbpmError};
use super::{LOCAL_DB_PATH, LOCAL_INDEX_PATH, NBPM_WORK_CURR, NBPM_WORK_DIR};
use crate::core::{pkgdb::PkgInfo, PkgDb, Set, SetInfo};
use crate::repo::REPO_BIN_DIR;
use crate::{utils, TypeErr};

/// Read user input from command line in form of a `String`.
pub fn read_line(prompt: &str) -> Result<String, TypeErr> {
    print!("{}", prompt);
    stdout().flush()?;
    let mut line = String::new();
    let _n = stdin().read_line(&mut line)?;
    line = line.trim_end().to_string();
    Ok(line)
}

/// Given a `Set` and the `Config` for `nbpm`, the function loads the index
/// `PkgDb` (if `set` is `Universe`) or local db `PkgDb` (if `set` is `Local`).
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

/// Creates the working directory of nbpm according to `nbpm::NBPM_WORK_DIR`. If the directory
/// already exits, this function does nothing. It also creates the current working directory
/// in `nbpm::NBPM_WORK_CURR`.
pub fn init_working_dir() -> Result<(), TypeErr> {
    if Path::new(NBPM_WORK_DIR).is_dir() {
        //if the directory exists, delete all its contents, as they are considered old
        if let Err(e) = fs::remove_dir_all(NBPM_WORK_DIR) {
            return Err(Box::new(e));
        }
    }

    if let Err(e) = fs::create_dir(NBPM_WORK_DIR) {
        return Err(Box::new(e));
    }

    // same with nbpm's current working directory
    if Path::new(NBPM_WORK_CURR).is_dir() {
        clean_work_curr()?;
    }
    if let Err(e) = fs::create_dir(NBPM_WORK_CURR) {
        return Err(Box::new(e));
    }
    Ok(())
}

/// Cleans all the contents of `NBPM_WORK_CURR` directory.
pub fn clean_work_curr() -> Result<(), TypeErr> {
    if let Err(e) = fs::remove_dir_all(NBPM_WORK_CURR) {
        return Err(Box::new(e));
    }

    if let Err(e) = fs::create_dir(NBPM_WORK_CURR) {
        return Err(Box::new(e));
    }
    Ok(())
}

/// Downloads all the packages listed in the given graph to the `NBPM_WORK_DIR` path. `config` is
/// also needed in order to get the url of the repository to install the packages from.
///
/// In the case of successfull download of all packages, the function returns a list of tuples.
/// Each tuple contains the name of the package and the path to the downloaded package.
///
/// # Errors
///
/// In case of failing download any package, the function returns an error describing the cause of
/// the download failure, from more datails see `utils::download`.
pub fn download_pkgs_to_workdir(
    graph: &HashMap<String, &PkgInfo>,
    config: &Config,
) -> Result<Vec<(String, String)>, TypeErr> {
    // initialize the working directory
    init_working_dir()?;

    // download all the packages to be installed
    let mut downl_files = vec![];
    for (name, info) in graph {
        //  get the location of the package in the server
        let pkg_loc = match info.set_info() {
            Some(set) => match set {
                SetInfo::Universe(u) => u.location(),
                SetInfo::Local(_) => unimplemented!(),
            },
            None => continue, // if the package is a metapackage
        };

        // name of the compressed package
        let pkg_xz_name = format!("{}.tar.xz", name);
        // the url to download the package from
        let pkg_url = format!(
            "{}/{}/{}/{}",
            config.repo_url(),
            REPO_BIN_DIR,
            pkg_loc,
            pkg_xz_name
        );
        // final path where the compressed package will be downloaded to
        let pkg_xz_path = format!("{}/{}", NBPM_WORK_DIR, pkg_xz_name);

        println!("[*] Downloanding: {}", pkg_url);
        utils::download(&pkg_url, Path::new(&pkg_xz_path))?;
        downl_files.push((name.clone(), pkg_xz_path));
    }
    Ok(downl_files)
}

/// Removes the packages already installed on the system (this info isobtained from the given
/// `PkgDb`) from the given packages graph. This function also lists the names, the action nbpm
/// will take and basic info about the packages that remain in the graph.
///
/// # Error
///
/// If a package from the the given `graph` request the downgrade of a package already installed
/// on the system, the function return a `NbpmError::RequiresPkgDowngrade` error.
pub fn purge_already_installed(
    graph: &mut HashMap<String, &PkgInfo>,
    db: &PkgDb,
) -> Result<(), TypeErr> {
    let mut not_install = vec![]; // list of packages already installed and to be skipped
    for (name, info) in graph.iter() {
        match db.get_pkg_info(name) {
            Some(local_pkg_info) => {
                // there is a package with the same name already installed in the system.
                // Determine if the package has to be updated or if the installation of this
                // package should be skipped.
                let curr_ver = local_pkg_info.version(); // current version of the package
                let new_ver = info.version();
                match new_ver.cmp(curr_ver) {
                    // a package with the same name and versions exits in the system, so skip the
                    // instalation of this package as it is already installed
                    Ordering::Equal => not_install.push(name.to_string()),
                    // cannot replace a package with an older version of a package
                    Ordering::Less => {
                        return Err(Box::new(NbpmError::RequiresPkgDowngrade(
                            name.to_string(),
                            curr_ver.clone(),
                            info.version().clone(),
                        )))
                    }
                    // every thing is ok, just update the package to a newer version of it
                    Ordering::Greater => println!(
                        "    {} {}    update {} -> {}",
                        name, info, curr_ver, new_ver,
                    ),
                }
            }
            // there is no package with the same name in the local PkgDb
            None => println!("    {} {}    install", name, info),
        }
    }
    // delete already installed packages from the graph
    for name in &not_install {
        let _ = graph.remove_entry(name);
    }
    Ok(())
}
