use crate::common::{self, CerberusError, CerberusResult};
use std::process;
use std::process::Command;

fn check_requirements() -> CerberusResult<()> {
    let result = Command::new("rsync").arg("--help").output();

    if result.is_err() {
        return Err(CerberusError::user_fault(
            "rsync must be installed for the backup command to work",
        ));
    }

    Ok(())
}

pub fn run(global: &clap::ArgMatches, params: &clap::ArgMatches) -> CerberusResult<()> {
    check_requirements()?;

    let destination_directory = params
        .value_of("destination-directory")
        .expect("Already check by clap");

    let hosts = common::list_hosts(global);
    let host = hosts
        .first()
        .expect("Safe because list_hosts is always non-empty");

    let remote_user_opt = params.value_of("remote-user");
    let mut source_directory = params
        .value_of("source-directory")
        .expect("Already check by clap")
        .to_owned();

    if *host != "localhost" || *host != "127.0.0.1" {
        source_directory = format!("{}:{}", host, source_directory);

        if let Some(remote_user) = remote_user_opt {
            source_directory = format!("{}@{}", remote_user, source_directory);
        }
    }

    // Targeted files for a proper eventstore backup.
    // https://eventstore.org/docs/server/database-backup/index.html
    let targets = ["*.chk", "index", "*.0*"];

    for target in targets.iter() {
        if *target == "*.chk" {
            println!("Backing up pointer files…");
        }

        if *target == "index" {
            println!("Backing up index folder…");
        }

        if *target == "*.0*" {
            println!("Backing up chunk files…");
        }

        let output = process::Command::new("rsync")
            .arg("-a")
            .arg("-v")
            .arg(format!("{}/{}", source_directory, target))
            .arg(destination_directory)
            .status()?;

        if !output.success() {
            return Err(CerberusError::user_fault("Backup failed"));
        }
    }

    Ok(())
}
