#[macro_use]
extern crate clap;

use semver::VersionReq;

use std::error::Error;
use std::process::exit;

pub mod core;
pub mod nbpm;
pub mod repo;
pub mod utils;

// custom types
pub type TypeErr = Box<dyn Error>;
pub type Query = (String, VersionReq);
pub type Dependencies = Option<Vec<Query>>;

// declare constants
pub const DEFAULT_SET: core::Set = core::Set::Universe;

pub fn exit_with_err(err: Box<dyn Error>) -> ! {
    eprintln!("Error: {}", err);
    exit(1);
}
