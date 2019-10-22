use crate::common::{ CerberusResult, CerberusError, NodeInfo };
use futures::future::Future;

pub fn run(global: &clap::ArgMatches, _: &clap::ArgMatches)
    -> CerberusResult<()>
{
    let base_url = crate::common::create_node_uri(global);
    let connection = crate::common::create_connection(global, |builder|
    {
        builder.connection_retry(eventstore::Retry::Only(0))
    })?;

    println!("Checking public HTTP port…");

    let req = reqwest::Client::new()
        .get(&format!("{}/info?format=json", base_url));

    let mut resp = req.send().map_err(|e|
    {
        CerberusError::UserFault(
            format!("Unable to confirm public HTTP port confirmation on {}: {}", base_url, e))
    })?;

    let info: NodeInfo = resp.json().map_err(|e|
        CerberusError::DevFault(
            format!("Failed to parse NodeInfo: {}", e))
    )?;

    let host = crate::common::node_host(global);
    let http_port = crate::common::public_http_port(global);
    let tcp_port = crate::common::public_tcp_port(global);

    println!(
        "EventStore Node HTTP port available on {}:{}\n\
        version: {}\nstate: {}\n", host, http_port, info.version, info.state);

    println!("Checking public TCP port…");

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

    println!("Successfully connect to node public TCP port on {}:{}", host, tcp_port);

    Ok(())
}
