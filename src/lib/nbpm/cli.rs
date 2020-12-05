use clap::{App, Arg};

pub fn init_cli_args() -> App<'static, 'static> {
    App::new("nbpm")
        .author(crate_authors!())
        .about("Nebula package manager")
        .version(crate_version!())
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .takes_value(true)
                .value_name("path")
                .help("read the configuration file from a custom path"),
        )
        .arg(
            Arg::with_name("update-repos")
                .short("u")
                .long("update")
                .conflicts_with_all(&["search", "install", "PKG"])
                .takes_value(false)
                .help("Update repostories"),
        )
        .arg(
            Arg::with_name("search")
                .short("s")
                .long("search")
                .takes_value(true)
                .value_name("package")
                .conflicts_with_all(&["update-repos", "install"])
                .help("Search for a package matching PKG"),
        )
        .arg(
            Arg::with_name("install")
                .short("i")
                .long("install")
                .takes_value(true)
                .multiple(true)
                .value_name("packages")
                .conflicts_with_all(&["update-repos", "search"])
                .help("Install a package or list of packages"),
        )
}
