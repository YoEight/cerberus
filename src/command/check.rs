use crate::common::{ CerberusResult, CerberusError, NodeInfo, ClusterMembers };
use futures::future::Future;

pub fn run(global: &clap::ArgMatches, _: &clap::ArgMatches)
    -> CerberusResult<()>
{
    let base_url = crate::common::create_node_uri(global);

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

    println!(
        ">> EventStore Node HTTP port available on {}:{}\n\
        version: {}\nstate: {}\n", host, http_port, info.version, info.state);

    println!("Checking if the node belongs to a cluster…");

    let req = reqwest::Client::new()
        .get(&format!("{}/gossip?format=json", base_url));

    let mut resp = req.send().map_err(|e|
    {
        CerberusError::UserFault(
            format!("Failed to read gossip endpoint on {}: {}", base_url, e))
    })?;

    if resp.status().is_client_error() {
        println!(">> The node doesn't appear to belong to a cluster");
    }

    if resp.status().is_server_error() {
        println!(
            ">> The node appears to belong to a cluster but is unabled but \
            is unabled to provide cluster information. You should worry!");
    }

    if resp.status().is_success() {
        let mut tcp_port = 0u16;
        let result: ClusterMembers = resp.json().map_err(|e|
        {
            CerberusError::UserFault(
                format!("Failed to deserialize ClusterMembers object: {}", e))
        })?;

        println!(">> Your instances belongs to a cluster of {} nodes", result.members.len());

        for member in result.members {
            println!("--------------------------------------------------");
            println!("Node: {}", member.external_tcp_ip);
            println!("Public TCP: {}", member.external_tcp_port);
            println!("Internal TCP: {}", member.internal_tcp_port);
            println!("Public HTTP: {}", member.external_http_port);
            println!("Internal HTTP: {}", member.internal_http_port);
            println!("State: {}", member.state);
            println!("Alive: {}", member.is_alive);
            println!("");

            if member.external_tcp_ip == host {
                tcp_port = member.external_tcp_port;
            }
        }

        println!("");

        let endpoint = format!("{}:{}", host, tcp_port)
            .parse()
            .unwrap();

        let connection = eventstore::Connection::builder()
            .connection_retry(eventstore::Retry::Only(0))
            .single_node_connection(endpoint);

        check_single_node_connection(global, tcp_port, connection)
    } else {
        let tcp_port = crate::common::public_tcp_port(global);
        let connection = crate::common::create_connection(global, |builder|
        {
            builder.connection_retry(eventstore::Retry::Only(0))
        })?;

        check_single_node_connection(global, tcp_port, connection)
    }

}

fn check_single_node_connection(
    global: &clap::ArgMatches,
    tcp_port: u16,
    connection: eventstore::Connection,
) -> CerberusResult<()> {

    println!("Checking public TCP port…");

    let host = crate::common::node_host(global);


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
                        format!("Failed to connect to database on {}:{}", host, tcp_port)));
        }
    }

    println!(">> Successfully connect to node public TCP port on {}:{}\n", host, tcp_port);

    Ok(())
}

