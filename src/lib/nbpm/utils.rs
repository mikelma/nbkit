use walkdir::WalkDir;

use std::collections::HashMap;
use std::fs;
use std::io::{stdin, stdout, Write};
use std::path::Path;

use super::{config::Config, NbpmError};
use super::{LOCAL_DB_PATH, LOCAL_INDEX_PATH, NBPM_WORK_CURR, NBPM_WORK_DIR};
use crate::core::{pkgdb::PkgInfo, PkgDb, Set, SetInfo};
use crate::repo::{REPO_BIN_DIR, REPO_PKG_INFO};
use crate::{utils, TypeErr};

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

/// Read user input from command line in form of a `String`.
pub fn read_line(prompt: &str) -> Result<String, TypeErr> {
    print!("{}", prompt);
    stdout().flush()?;
    let mut line = String::new();
    let _n = stdin().read_line(&mut line)?;
    line = line.trim_end().to_string();
    return Ok(line);
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
                if new_ver == curr_ver {
                    // a package with the same name and versions exits in the system, so skip the
                    // instalation of this package as it is already installed
                    not_install.push(name.to_string());
                } else if new_ver < curr_ver {
                    return Err(Box::new(NbpmError::RequiresPkgDowngrade(
                        name.to_string(),
                        curr_ver.clone(),
                        info.version().clone(),
                    )));
                } else {
                    println!(
                        "    {} {}    update {} -> {}",
                        name, info, curr_ver, new_ver,
                    );
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

/// Given a vector of tuples containing package names and paths to the compressed packages, the
/// function installs this compressed packages on the system.
///
/// If all packages are installed successfully function returns an `Ok` containing a `HashMap` with
/// the names and `PkgInfo` objects of the installed packages.
/// If the function fails to install a package, the installatioon of other packages is cancelled
/// and the error is returned together with the `HashMap` with the name and `PkgInfo` objects of
/// the successfully installed packages before the error ocurred.
///
/// # Errors
/// The function returns an error in the following cases:
///
/// - The path to the compressed package is invalid.
/// - Cannot decompress the package.
/// - Cannot read or deserialize the `pkginfo` file of the decompressed package.
/// - Cannot install package's files to the destination.
/// - Cannot clean the installation working directory.
pub fn install_pkgs(
    pkg_paths: &Vec<(String, String)>,
    config: &Config,
) -> Result<HashMap<String, PkgInfo>, (HashMap<String, PkgInfo>, TypeErr)> {
    let mut installed_pkgs = HashMap::new();
    for (pkg_name, path) in pkg_paths {
        println!("\n[*] Decompressing {}...", path);
        // decompress the downloaded package in nbpm's current working dir
        if let Err(e) = utils::run_cmd("tar", &["xvf", path.as_str(), "-C", NBPM_WORK_CURR]) {
            return Err((installed_pkgs, e));
        }

        // read and deserialize the info file of the package
        let info_str = match fs::read_to_string(format!("{}/{}", NBPM_WORK_CURR, REPO_PKG_INFO)) {
            Ok(v) => v,
            Err(e) => return Err((installed_pkgs, Box::new(e))),
        };
        let mut pkg_info = match toml::from_str::<HashMap<String, PkgInfo>>(&info_str) {
            Ok(v) => v,
            Err(e) => return Err((installed_pkgs, Box::new(e))),
        };

        println!("[*] Installing {}...", pkg_name);
        // installl all the files of the package
        if let Err(e) = install_pkg_files(NBPM_WORK_CURR, config.root()) {
            return Err((installed_pkgs, e));
        }

        // it's safe to call unwrap here as in the lines above, key's existance its ensured
        let info = pkg_info.remove(pkg_name).unwrap(); // get the `PkgInfo` object
        installed_pkgs.insert(pkg_name.clone(), info);

        // clean the installation working directory to be used with other package
        if let Err(e) = clean_work_curr() {
            return Err((installed_pkgs, e));
        }
    }
    Ok(installed_pkgs)
}
