#[macro_use]
extern crate clap;

use clap::{App, Arg};
use semver::Version;
use toml;
use walkdir::WalkDir;

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{stdin, stdout, Write};
use std::path::Path;

use nbkit::{
    core::pkgdb::{InfoLocal, PkgInfo, SetInfo},
    core::wrappers::{DependencyWrap, VersionWrap},
    utils, Query,
};

fn main() {
    let args = App::new("nbinfo-gen")
        .author(crate_authors!())
        .about("Helper program for creating nebula package's pkginfo file")
        .version(crate_version!())
        .arg(
            Arg::with_name("target-paths")
                .short("p")
                .long("pkg-paths")
                .takes_value(true)
                .value_name("paths")
                .help("files or directories of the package")
                .multiple(true)
                .required(true),
        )
        .get_matches();

    let mut paths = vec![];
    for p in args.values_of("target-paths").unwrap() {
        let path = Path::new(p);

        if !path.exists() {
            panic!("path does not exist");
        } else if path.is_file() {
            paths.push(p.to_string());
        } else if path.is_dir() {
            for entry in WalkDir::new(path).contents_first(true) {
                paths.push(match entry {
                    Ok(v) => format!("{}", v.path().display()),
                    Err(_) => panic!("path error"),
                });
            }
        }
    }

    let mut name = None;
    let mut version = None;
    let mut description = None;
    let mut depends: Option<Vec<DependencyWrap>> = None;

    loop {
        println!("---------------------------------------");
        println!("(1) Name: {:?}", name);
        println!("(2) Version: {:?}", version);
        println!("(3) Description: {:?}", description);
        println!("(4) Add dependency: {:?}\n", depends);
        println!("(0) Done\n");
        println!("---------------------------------------");

        let n = utils::read_line("Select an action: ")
            .unwrap()
            .parse::<usize>()
            .unwrap();

        if n == 0 {
            if name.is_none() {
                eprintln!("Error: Name missing");
            } else if version.is_none() {
                eprintln!("Error: Version missing");
            } else if description.is_none() {
                eprintln!("Error: Description missing");
            }
            break;
        }
        match utils::read_line("Set new value: ") {
            Ok(v) => match n {
                1 => name = Some(v),
                2 => match Version::parse(&v) {
                    Ok(ver) => version = Some(ver),
                    Err(e) => eprintln!("Error: {}", e),
                },
                3 => description = Some(v),
                4 => match utils::parse_pkg_str_info(&v) {
                    Ok(q) => match &mut depends {
                        Some(d) => d.push(DependencyWrap::from(q)),
                        None => depends = Some(vec![DependencyWrap::from(q)]),
                    },
                    Err(e) => eprintln!("Error: {}", e),
                },
                _ => unreachable!(),
            },
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    let vreq = VersionWrap::from(version.unwrap());
    let setinfo = SetInfo::Local(InfoLocal::from(paths));
    let pkginfo = PkgInfo::from(vreq, depends, description.unwrap(), Some(setinfo));

    let mut info = HashMap::new();
    info.insert(name.unwrap(), pkginfo);

    let serialized = toml::to_string(&info).unwrap();

    let mut file = File::create("nbinfo.toml").unwrap();
    file.write_all(serialized.as_bytes()).unwrap();
}
