// use toml;

use std::io::{stdin, stdout, Write};
use std::path::Path;

use nbkit::core::{PkgDb, SetInfo};
use nbkit::nbpm::*;
use nbkit::{exit_with_err, repo::*, utils};

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
        let index_db = get_index_pkgdb(&config);
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
        let index_db = get_index_pkgdb(&config);
        let names: Vec<&str> = names_list.collect();
        let graph = match index_db.get_subgraph(Some(names)) {
            Ok(g) => g,
            Err(e) => exit_with_err(e),
        };

        println!("Packages to be installed ({}):", graph.len());
        for (name, info) in &graph {
            println!("    {} {}", name, info);
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

        // TODO: Lock the database file
        // open the local package database
        let mut local_db = get_local_pkgdb(&config);

        if let Err(e) = init_working_dir() {
            exit_with_err(e);
        }

        // download all the packages to be installed
        let mut pkg_paths = vec![];
        for (name, info) in graph {
            //  get the location of the package in the server
            let pkg_loc = match info.get_set_info() {
                Some(set) => match set {
                    SetInfo::Universe(u) => u.location(),
                    SetInfo::Local(_) => unimplemented!(),
                },
                None => continue,
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

            pkg_paths.push(pkg_xz_path);
        }
    }

    // -------------------------------- //
}

fn get_index_pkgdb(config: &Config) -> PkgDb {
    let index_path = format!("{}/{}", config.home(), LOCAL_INDEX_PATH);
    match PkgDb::load(Path::new(&index_path)) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to load repository index from {}.", index_path);
            exit_with_err(e)
        }
    }
}

fn get_local_pkgdb(config: &Config) -> PkgDb {
    let local_db_path = format!("{}/{}", config.home(), LOCAL_DB_PATH);
    match PkgDb::load(Path::new(&local_db_path)) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Failed to load local db from: {}", local_db_path);
            exit_with_err(e)
        }
    }
}
