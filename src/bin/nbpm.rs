use std::fs;
use std::path::Path;

use nbkit::core::{PkgDb, Set};
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

    // a closure to save the local `PkgDb` if it's changed
    let save_local_db = |db_ref: &PkgDb| {
        let db_path = format!("{}/{}", config.home(), LOCAL_DB_PATH);
        match toml::to_string_pretty(db_ref) {
            Ok(s) => {
                if let Err(e) = fs::write(db_path, s.as_bytes()) {
                    exit_with_err(Box::new(e));
                }
            }
            Err(e) => {
                exit_with_err(Box::new(e));
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

        // TODO: Lock the database file
        // open the local package database
        let mut local_db = match nbpm::utils::load_pkgdb(&config, Set::Local) {
            Ok(v) => v,
            Err(e) => exit_with_err(Box::new(e)),
        };

        if let Err(e) = nbpm::install::install_handler(&names, &config, &mut local_db, &index_db) {
            eprintln!("[!] Installation failed");
            exit_with_err(e);
        }
        save_local_db(&local_db);
    }
    // -------------------------------- //

    // ------------ remove ------------ //
    if let Some(sub_cmd) = args.subcommand_matches("remove") {
        let names_list = sub_cmd.values_of("packages").unwrap();
        // TODO: Lock the database file
        // open the local package database
        let mut local_db = match nbpm::utils::load_pkgdb(&config, Set::Local) {
            Ok(v) => v,
            Err(e) => exit_with_err(Box::new(e)),
        };

        let to_remove_names: Vec<&str> = names_list.collect();
        if let Err(e) = nbpm::remove::remove_handler(
            &to_remove_names,
            sub_cmd.is_present("recursive"),
            true, // ask for user confirmation before removing the packages
            true, // check for conflicts
            &mut local_db,
        ) {
            exit_with_err(e);
        }
        save_local_db(&local_db);
    }
}
