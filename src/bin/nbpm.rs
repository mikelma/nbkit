use std::fs;
use std::io::{stdin, stdout, Write};
use std::path::Path;

use nbkit::core::{pkgdb::SetInfo, Set};
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
        match index_db.get_pkg_info(pkg_name) {
            Some(info) => println!(
                "{} - {}    {}",
                pkg_name,
                info.version(),
                info.description()
            ),
            None => {
                eprintln!("Package {} not found =(", pkg_name);
            }
        }
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

        // remove the already installed packages from the graph, this function will also show the
        // action nbpm will take for every package (install/update...)
        if let Err(e) = nbpm::utils::purge_already_installed(&mut graph, &local_db) {
            exit_with_err(e);
        }

        // after pkg graph purge, check if there is any package to be installed
        if graph.is_empty() {
            println!("Packages already installed. Skipping the installation...");
            return;
        }

        // show the packages to be installed and askfor user confirmation
        println!("Packages to be installed ({}):", graph.len());
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

        if let Err(e) = nbpm::utils::init_working_dir() {
            exit_with_err(e);
        }

        let downl_files = match nbpm::utils::download_pkgs_to_workdir(&graph, &config) {
            Ok(v) => v,
            Err(e) => exit_with_err(e),
        };

        match nbpm::utils::install_pkgs(&downl_files, &config) {
            Ok(installed_pkgs) => {
                // successfull installation of all the packages, now update the local_db
                for (name, mut info) in installed_pkgs {
                    // set the prefix of the package's file paths to the root path specified in the
                    // config file
                    match info.mut_set_info() {
                        Some(SetInfo::Local(set)) => set.set_path_prefix(Path::new(config.root())),
                        Some(SetInfo::Universe(_)) => unreachable!(),
                        None => (), // the package is a meta-package, it does not contain any Local set info to modify
                    }
                    let _ = local_db.insert(&name, info);
                }
            }
            Err((installed_pkgs, err)) => {
                eprintln!("[EE] Installation failed: {}", err);
                // delete all the installed packages.
                if let Err(e) = nbpm::utils::remove_local_pkgs(&installed_pkgs, &config) {
                    eprintln!("[!] Warning failed to remove package: {}", e);
                    exit_with_err(e);
                }
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

        // save the updated local db with the new packages
        let db_path = format!("{}/{}", config.home(), LOCAL_DB_PATH);
        match toml::to_string_pretty(&local_db) {
            Ok(s) => {
                if let Err(e) = fs::write(db_path, s.as_bytes()) {
                    exit_with_err(Box::new(e));
                }
            }
            Err(e) => {
                exit_with_err(Box::new(e));
            }
        }

        println!("Done!");
    }
    // -------------------------------- //

    // ------------ remove ------------ //
    if let Some(names_list) = args.values_of("remove") {
        println!("Remove: {:?}", names_list);
    }
    // -------------------------------- //
}
