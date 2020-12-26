use std::fs;
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
        let mut graph = match index_db.get_subgraph(Some(&names), true) {
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
        match nbpm::utils::read_line("\nAre you sure you want to install this packages? [Y/n] ") {
            Ok(line) => {
                if !line.is_empty() && line != "y" && line != "Y" {
                    println!("Operation cancelled");
                    std::process::exit(0);
                }
            }
            Err(e) => exit_with_err(e),
        }

        if let Err(e) = nbpm::install::install_handler(&graph, &config, &mut local_db) {
            eprintln!("[!] Installation failed");
            exit_with_err(e);
        }

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
    if let Some(sub_cmd) = args.subcommand_matches("remove") {
        let names_list = sub_cmd.values_of("packages").unwrap();
        // TODO: Lock the database file
        // open the local package database
        let mut local_db = match nbpm::utils::load_pkgdb(&config, Set::Local) {
            Ok(v) => v,
            Err(e) => exit_with_err(Box::new(e)),
        };
        let to_remove_names: Vec<&str> = names_list.collect();

        let to_remove_graph =
            match local_db.get_subgraph(Some(&to_remove_names), sub_cmd.is_present("recursive")) {
                Ok(g) => g,
                Err(e) => exit_with_err(e),
            };

        println!(
            "The following packages are going to be removed ({}):",
            to_remove_graph.len()
        );
        to_remove_graph
            .iter()
            .for_each(|(name, info)| println!("     {} {}", name, info.version()));

        // ask the user for confirmation before removing the packages
        match nbpm::utils::read_line("\nAre you sure you want to remove this packages? [Y/n] ") {
            Ok(line) => {
                if !line.is_empty() && line != "y" && line != "Y" {
                    println!("Operation cancelled");
                    std::process::exit(0);
                }
            }
            Err(e) => exit_with_err(e),
        }

        println!("[*] Checking for conflicts...");
        if let Err(e) = local_db.check_remove(to_remove_names) {
            exit_with_err(e);
        }

        for (name, info) in &to_remove_graph {
            println!("[*] Removing {}...", name);
            let files = match info.set_info() {
                Some(SetInfo::Local(local_info)) => local_info.paths(),
                Some(SetInfo::Universe(_)) => unreachable!(),
                None => continue,
            };
        }
    }
}
