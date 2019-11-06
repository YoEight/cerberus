use crate::common::{ CerberusResult, CerberusError };

pub fn run(
    global: &clap::ArgMatches,
) -> CerberusResult<()> {
    println!("Hello from dashboard!");
    Ok(())
}
