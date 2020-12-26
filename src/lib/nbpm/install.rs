use walkdir::WalkDir;

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::utils::{
    clean_work_curr, download_pkgs_to_workdir, remove_local_pkg_files, remove_path,
};
use super::NBPM_WORK_CURR;
use super::{Config, NbpmError};
use crate::core::{pkgdb::PkgInfo, PkgDb, SetInfo};
use crate::repo::REPO_PKG_INFO;
use crate::{utils, TypeErr};

/// Given a vector of tuples containing package names and paths to the compressed packages, the
/// function installs this compressed packages on the system and updates the local `PkgDb`.
///
/// **NOTE**: You might want to call `nbpm::utils::purge_already_installed` before this function.
/// In order to avoid installing already installed packages.
///
/// # Errors
/// The function returns an error in the following cases:
///
/// - The path to the compressed package is invalid.
/// - Cannot decompress the package.
/// - Cannot read or deserialize the `pkginfo` file of the decompressed package.
/// - Cannot install package's files to the destination.
/// - Cannot clean the installation working directory.
pub fn install_handler(
    graph: &HashMap<String, &PkgInfo>,
    config: &Config,
    local_db: &mut PkgDb,
) -> Result<(), TypeErr> {
    let downl_files = download_pkgs_to_workdir(&graph, &config)?;

    let mut installed_pkgs = vec![]; // names of the installed packages
    let mut status: Result<(), TypeErr> = Ok(());
    for (pkg_name, path) in downl_files {
        println!("\n[*] Decompressing {}...", path);
        // decompress the downloaded package in nbpm's current working dir
        if let Err(e) = utils::run_cmd("tar", &["xvf", path.as_str(), "-C", NBPM_WORK_CURR]) {
            status = Err(e);
            break;
        }

        // read and deserialize the info file of the package
        let info_str = match fs::read_to_string(format!("{}/{}", NBPM_WORK_CURR, REPO_PKG_INFO)) {
            Ok(v) => v,
            Err(e) => {
                status = Err(Box::new(e));
                break;
            }
        };
        let mut pkg_info = match toml::from_str::<HashMap<String, PkgInfo>>(&info_str) {
            Ok(v) => v,
            Err(e) => {
                status = Err(Box::new(e));
                break;
            }
        };

        // it's safe to call unwrap here as in the lines above, key's existance its ensured
        let mut info = pkg_info.remove(&pkg_name).unwrap(); // get the `PkgInfo` object

        // set the prefix of the package's file paths to the root path specified in the
        // config file
        match info.mut_set_info() {
            Some(SetInfo::Local(set)) => set.set_path_prefix(Path::new(config.root())),
            Some(SetInfo::Universe(_)) => unreachable!(),
            None => (), // the package is a meta-package, it does not contain any Local set info to modify
        }
        let _ = local_db.insert(&pkg_name, info);
        println!("[*] Installing {}...", pkg_name);
        installed_pkgs.push(pkg_name);

        // installl all the files of the package
        if let Err(e) = install_pkg_files(NBPM_WORK_CURR, config.root()) {
            status = Err(e);
            break;
        }

        // clean the installation working directory to be used with other package
        if let Err(e) = clean_work_curr() {
            status = Err(e);
            break;
        }
    }

    // get metapackages of the graph and insert them into the local db as they are considered
    // installed on the system
    graph
        .iter()
        .filter(|(_, &info)| info.is_meta())
        .for_each(|(name, &info)| {
            let _ = local_db.insert(name, info.clone());
        });

    if status.is_err() {
        // something went wront
        println!(
            "\n[!] Trying to undo the installation... {:?}",
            installed_pkgs
        );
        let names_list: Vec<&str> = installed_pkgs.iter().map(|s| s.as_str()).collect();
        let installed_graph = local_db.get_subgraph(Some(&names_list), false)?;
        remove_local_pkg_files(&installed_graph)?;
    }
    status
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
