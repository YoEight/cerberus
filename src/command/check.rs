use crate::common::{CerberusError, CerberusResult};

use crate::api::{Api, ClusterState};

pub async fn run(
    global: &clap::ArgMatches<'_>,
    params: &clap::ArgMatches<'_>,
    api: Api<'_>,
) -> CerberusResult<()> {
    println!("Checking public HTTP port…");

    let info = api
        .node_info()
        .await
        .map_err(|error| match error.downcast::<CerberusError>() {
            Ok(tpe) => match *tpe {
                CerberusError::UserFault(_) => CerberusError::user_fault(format!(
                    ">> Cannot connect to node [{}:{}]",
                    api.host(),
                    api.port()
                )),

                same => same.boxed(),
            },

            Err(same) => same,
        })?;

    println!(
        ">> EventStore Node HTTP port available on {}:{}\n\
        version: {}\nstate: {}\n",
        api.host(),
        api.port(),
        info.version,
        info.state
    );

    if !params.is_present("no-cluster-check") {
        println!("Checking if the node belongs to a cluster…");

        let state = api.gossip().await?;

        match state {
            ClusterState::NoCluster => {
                println!(">> The node doesn't appear to belong to a cluster\n")
            }

            ClusterState::ProblematicClusterNode => println!(
                ">> The node appears to belong to a cluster but is unabled but \
                    is unabled to provide cluster information. You should worry!\n"
            ),

            ClusterState::Cluster(cluster) => {
                println!(
                    ">> Your instance belongs to a cluster of {} nodes",
                    cluster.members.len()
                );

                let mut tcp_port = 0u16;

                for member in cluster.members {
                    println!("--------------------------------------------------");
                    print!("Node: {}", member.external_tcp_ip);

                    if member.external_tcp_ip == api.host()
                        && member.external_http_port == api.port()
                    {
                        tcp_port = member.external_tcp_port;
                        println!("\nVersion: {}", info.version);
                    } else {
                        let node_api = api.with_different_node(
                            &member.external_http_ip,
                            member.external_http_port,
                        );

                        if let Ok(node_info) = node_api.node_info().await {
                            println!("\nVersion: {}", node_info.version);
                        } else {
                            println!(" [Not available]");
                        }
                    }

                    println!("Public TCP: {}", member.external_tcp_port);
                    println!("Internal TCP: {}", member.internal_tcp_port);
                    println!("Public HTTP: {}", member.external_http_port);
                    println!("Internal HTTP: {}", member.internal_http_port);
                    println!("State: {}", member.state);
                    println!("Alive: {}", member.is_alive);
                    println!();
                }

                println!();

                let endpoint = format!("{}:{}", api.host(), tcp_port).parse().unwrap();

                let connection = eventstore::Connection::builder()
                    .connection_retry(eventstore::Retry::Only(0))
                    .single_node_connection(endpoint)
                    .await;

                return check_single_node_connection(global, tcp_port, connection).await;
            }
        };
    }

    let tcp_port = crate::common::public_tcp_port(global);
    let connection = crate::common::create_connection(global, |builder| {
        builder.connection_retry(eventstore::Retry::Only(0))
    })
    .await?;

    check_single_node_connection(global, tcp_port, connection).await
}

async fn check_single_node_connection(
    global: &clap::ArgMatches<'_>,
    tcp_port: u16,
    connection: eventstore::Connection,
) -> CerberusResult<()> {
    println!("Checking public TCP port…");

    let host = crate::common::node_host(global);

    let result = connection
        .read_all()
        .start_from_beginning()
        .max_count(1)
        .execute()
        .await;

    if let Err(e) = result {
        if let eventstore::OperationError::Aborted = e {
            return Err(CerberusError::user_fault(format!(
                "Failed to connect to database on {}:{}",
                host, tcp_port
            )));
        }
    }

    println!(
        ">> Successfully connect to node public TCP port on {}:{}\n",
        host, tcp_port
    );

    Ok(())
}
