use std::collections::HashMap;
use std::fs;
use std::io::{stdin, stdout, Write};
use std::path::Path;

use nbkit::core::{pkgdb::PkgInfo, Set, SetInfo};
use nbkit::nbpm::{self, *};
use nbkit::{repo::*, utils};

fn main() {
    let args = cli::init_cli_args().get_matches();

    // load the configuration
    let config = match args.value_of("config") {
        // a custom configuration file path has been given
        Some(path) => match Config::from(Path::new(path)) {
            Ok(c) => c,
            Err(e) => exit_with_err(e),
        },
        // if no custom path is given, the default path is used
        None => {
            let default = format!("{}/{}", DEF_NBPM_PATH, NBPM_CONFIG_FILE);
            match Config::from(Path::new(&default)) {
                Ok(c) => c,
                // if loading from the default path fails, go to default values
                Err(e) => {
                    eprintln!(
                        "Warning: Cannot load configuration from default path {}: {}",
                        default, e
                    );
                    eprintln!("Warning: Loading deafult configurations");
                    Default::default()
                }
            }
        }
    };

    // ------------ update ------------ //
    if args.is_present("update-repos") {
        // full url to the remote repository index
        let index_url = format!("{}/{}", config.repo_url(), REPO_INDEX_PATH);
        println!("Updating repo index from: {}", index_url);

        // path to store the new index db
        let index_path = format!("{}/{}", config.home(), LOCAL_INDEX_PATH);

        if let Err(e) = utils::download(&index_url, Path::new(&index_path)) {
            eprintln!("Cannot update repository index.");
            exit_with_err(e);
        }
        println!("Updating done!");
    }
    // -------------------------------- //

    // ------------ search ------------ //
    if let Some(pkg_name) = args.value_of("search") {
        let index_db = match nbpm::utils::load_pkgdb(&config, Set::Universe) {
            Ok(v) => v,
            Err(e) => exit_with_err(Box::new(e)),
        };
        let pkg_info = match index_db.get_pkg_info(pkg_name) {
            Ok(p) => p,
            Err(e) => exit_with_err(e),
        };
        println!(
            "{} - {}    {}",
            pkg_name,
            pkg_info.version(),
            pkg_info.description()
        );
    }
    // -------------------------------- //

    // ------------ install ----------- //
    if let Some(names_list) = args.values_of("install") {
        let index_db = match nbpm::utils::load_pkgdb(&config, Set::Universe) {
            Ok(v) => v,
            Err(e) => exit_with_err(Box::new(e)),
        };
        let names: Vec<&str> = names_list.collect();
        let mut graph = match index_db.get_subgraph(Some(names)) {
            Ok(g) => g,
            Err(e) => exit_with_err(e),
        };

        // TODO: Lock the database file
        // open the local package database
        let mut local_db = match nbpm::utils::load_pkgdb(&config, Set::Local) {
            Ok(v) => v,
            Err(e) => exit_with_err(Box::new(e)),
        };

        println!("Packages to be installed ({}):", graph.len());
        let mut not_install = vec![]; // list of packages already installed and to be skipped
        for (name, info) in &graph {
            if local_db.contains_name(name) {
                if local_db.contains(name, info.version()) {
                    // a package with the same name and versions exits in the system, so skip the
                    // instalation of this package as it is already installed
                    not_install.push(name.to_string());
                } else {
                    // a package with the same name exists in the local db,
                    // but versions are different, so update the package
                    println!("    {} {}    update", name, info);
                }
            } else {
                println!("    {} {}    install", name, info);
            }
        }

        // delete already installed packages from the graph
        for name in &not_install {
            let _ = graph.remove_entry(name);
        }

        let mut line = String::new();
        print!("\nAre you sure you want to install this packages? [Y/n] ");
        if let Err(e) = stdout().flush() {
            exit_with_err(Box::new(e));
        }
        let _n = stdin()
            .read_line(&mut line)
            .expect("Cannot read user input");
        line = line.trim_end().to_string();
        if line == "N" || line == "n" {
            println!("Installation cancelled");
            std::process::exit(0);
        }
        println!();

        if let Err(e) = init_working_dir() {
            exit_with_err(e);
        }

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
            if let Err(e) = utils::download(&pkg_url, Path::new(&pkg_xz_path)) {
                exit_with_err(e);
            }
            downl_files.push((name, pkg_xz_path));
        }

        // create a dir inside the working directory of nbpm to decompress the packages into
        let mut success = true;
        let mut installed_pkgs = HashMap::new();
        for (pkg_name, path) in downl_files {
            println!("\n[*] Decompressing {}...", path);
            // decompress the downloaded package in nbpm's current working dir
            if let Err(e) = utils::run_cmd("tar", &["xvf", path.as_str(), "-C", NBPM_WORK_CURR]) {
                exit_with_err(e);
            }

            // read the info file of the package
            let info_str = match fs::read_to_string(format!("{}/{}", NBPM_WORK_CURR, REPO_PKG_INFO))
            {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    success = false;
                    break;
                }
            };
            let mut pkg_info = match toml::from_str::<HashMap<String, PkgInfo>>(&info_str) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    success = false;
                    break;
                }
            };

            println!("[*] Installing {}...", pkg_name);
            if let Err(e) = nbpm::utils::install_pkg_files(NBPM_WORK_CURR, config.root()) {
                eprintln!("Error: {}", e);
                success = false;
                break;
            }

            let name = match pkg_info.keys().next() {
                Some(k) => k.clone(),
                None => unimplemented!(),
            };
            // it's safe to call unwrap here as in the lines above, key's existance its ensured
            let info = pkg_info.remove(&name).unwrap();
            installed_pkgs.insert(name.clone(), info);

            if let Err(e) = clean_work_curr() {
                eprintln!("Error: {}", e);
                success = false;
                break;
            }
        }

        // if the installation of ALL the packages has been successfull, update the local db (use `new_pkgs`), else,
        // delete all the installed packages.
        if !success {
            eprintln!("\n[EE] Installation failed =(");

            if let Err(e) = nbpm::utils::remove_local_pkgs(&installed_pkgs, &config) {
                eprintln!("[!] Warning failed to remove package: {}", e);
                exit_with_err(e);
            }
        } else {
            // success installation of all the packages, so update the local_db
            for (name, info) in installed_pkgs {
                let _ = local_db.insert(&name, info);
            }
            let index_path = format!("{}/{}", config.home(), LOCAL_DB_PATH);
            match toml::to_string_pretty(&local_db) {
                Ok(s) => {
                    if let Err(e) = fs::write(index_path, s.as_bytes()) {
                        exit_with_err(Box::new(e));
                    }
                }
                Err(e) => {
                    exit_with_err(Box::new(e));
                }
            }
        }
    }
    // -------------------------------- //
}
