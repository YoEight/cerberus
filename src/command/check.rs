use crate::common::{ CerberusResult, CerberusError };
use futures::future::Future;

pub fn run(global: &clap::ArgMatches, _: &clap::ArgMatches)
    -> CerberusResult<()>
{
    let connection = crate::common::create_connection(global, |builder|
    {
        builder.connection_retry(eventstore::Retry::Only(0))
    })?;

    println!("Checking…");

    let result = connection.read_all()
        .start_from_beginning()
        .max_count(1)
        .execute()
        .wait();

    if let Err(e) = result {
        if let eventstore::OperationError::Aborted = e {
            return
                Err(
                    CerberusError::UserFault(
                        "Failed to connect to database".to_owned()));
        }
    }

    println!("Successfully connect to the database");

    Ok(())
}
