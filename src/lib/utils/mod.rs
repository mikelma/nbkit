use reqwest;
use reqwest::StatusCode;
use semver::VersionReq;
use sha2::{Digest, Sha256};

use super::{core::NbError, Query, TypeErr};

use std::fs::{File, OpenOptions};
use std::io::{stdin, stdout, Read, Write};
use std::path::Path;

/// parse information of a package given a string. The string format must be: pkg_name or
/// [pkgname][comp_op][version]. Examples: "neofetch", "glibc", "linux>=5.5.3" and "make<1.0".
pub fn parse_pkg_str_info(text: &str) -> Result<Query, TypeErr> {
    // search for comparison operator on the query
    for operator in &["==", ">=", "<=", ">", "<"] {
        // if an operator is present extract the name, comparison operator and version
        if text.contains(operator) {
            // split the name and versionreq part
            let mut splitted = text.split(operator);
            // its safe to call unwrap here as the `splitted` will always
            // have at least one element
            let name = splitted.next().unwrap();
            let comp_ver = match splitted.next() {
                Some(s) => VersionReq::parse(s)?,
                None => VersionReq::any(),
            };
            return Ok((name.to_string(), comp_ver));
        }
    }
    Ok((text.to_string(), VersionReq::any()))
}

/// Downloas a file from the given `url` and saves it as `outpath`.
pub fn download(url: &str, outfile: &Path) -> Result<(), TypeErr> {
    // delete the file/dir to download if it already exists
    if outfile.is_dir() && outfile.exists() {
        std::fs::remove_dir_all(&outfile)?;
    } else if outfile.is_file() && outfile.exists() {
        std::fs::remove_file(&outfile)?;
    }

    let resp = reqwest::blocking::get(url)?;
    // check for errors
    let status = resp.status();
    if status.is_client_error() {
        return Err(Box::new(NbError::ClientError(status.to_string())));
    } else if status.is_server_error() {
        return Err(Box::new(NbError::ClientError(status.to_string())));
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(true)
        .open(&outfile)?;
    file.write_all(&resp.bytes()?)?;
    Ok(())
}

/// Computes the SHA256 hash of the file in the given path.
pub fn file2hash(filepath: &Path) -> Result<String, TypeErr> {
    let mut file = File::open(filepath)?;
    let mut buffer = Vec::<u8>::new();
    file.read_to_end(&mut buffer)?;
    Ok(format!("{:x}", Sha256::digest(&buffer)))
}

pub fn read_line(prompt: &str) -> Result<String, TypeErr> {
    let mut line = String::new();
    print!("\n{}", prompt);
    if let Err(e) = stdout().flush() {
        return Err(Box::new(e));
    }
    let _n = stdin()
        .read_line(&mut line)
        .expect("Cannot read user input");
    Ok(line.trim_end().to_string())
}
