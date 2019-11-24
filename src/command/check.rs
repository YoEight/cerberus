use crate::common::{ CerberusResult, CerberusError };

use crate::api::{
    Api,
    ClusterState,
};

use futures::future::Future;
use tui::backend::Backend;
use tui::layout::{ Constraint, Layout };
use tui::Terminal;
use tui::widgets::{ Block, Borders, Row, Table, Widget };

pub fn run<B>(
    mut terminal: Terminal<B>,
    global: &clap::ArgMatches,
    params: &clap::ArgMatches,
    api: Api,
) -> CerberusResult<()>
    where
        B: Backend
{
    println!("Checking public HTTP port…");

    let info = api.node_info().map_err(|tpe| {
        match tpe {
            CerberusError::UserFault(_) =>
                CerberusError::UserFault(
                    format!(
                        ">> Cannot connect to node [{}:{}]", api.host(), api.port())),

            same => same,
        }
    })?;

    println!(
        ">> EventStore Node HTTP port available on {}:{}\n\
        version: {}\nstate: {}\n", api.host(), api.port(), info.version, info.state);

    if !params.is_present("no-cluster-check") {
        println!("Checking if the node belongs to a cluster…");

        let state = api.gossip()?;

        match state {
            ClusterState::NoCluster =>
                println!(">> The node doesn't appear to belong to a cluster\n"),

            ClusterState::ProblematicClusterNode =>
                println!(
                    ">> The node appears to belong to a cluster but is unabled but \
                    is unabled to provide cluster information. You should worry!\n"),

            ClusterState::Cluster(cluster) => {
                println!(
                    ">> Your instance belongs to a cluster of {} nodes"
                    , cluster.members.len());

                let mut tcp_port = 0u16;

                let members_info = cluster.members.iter().map(|member| {
                    let api = api.with_different_node(
                        &member.external_http_ip,
                        member.external_http_port,
                    );

                    let status = if member.is_alive {
                        "Running"
                    } else {
                        "Down"
                    };

                    api.node_info().ok().map(|info| {
                        Row::Data(vec![
                            format!("{}", info.version),
                            member.external_http_ip.clone(),
                            member.state.clone(),
                            status.to_owned(),
                            format!("{}", member.external_tcp_port),
                            format!("{}", member.internal_tcp_port),
                            format!("{}", member.external_http_port),
                            format!("{}", member.internal_http_port),
                        ].into_iter())
                    })
                }).flatten();

                //terminal.clear().unwrap();
                terminal.draw(|mut frame| {
                    let mut size = frame.size();

                    let headers = [
                        "Version",
                        "Ip",
                        "State",
                        "Status",
                        "External TCP port",
                        "Internal TCP port",
                        "External HTTP port",
                        "Internal HTTP port",
                    ];

                    let rects = Layout::default()
                        .constraints([Constraint::Percentage(100)].as_ref())
                        .margin(5)
                        .split(frame.size());

                    Table::new(headers.into_iter(), members_info)
                        .block(Block::default().borders(Borders::NONE).title("Cluster Info"))
                        .widths(&[20, 20, 20, 20, 20, 20, 20, 20])
                        .render(&mut frame, rects[0]);
                }).unwrap();

                println!();

                let endpoint = format!("{}:{}", api.host(), tcp_port)
                    .parse()
                    .unwrap();

                let connection = eventstore::Connection::builder()
                    .connection_retry(eventstore::Retry::Only(0))
                    .single_node_connection(endpoint);

                return check_single_node_connection(global, tcp_port, connection);
            },
        };
    }

    let tcp_port = crate::common::public_tcp_port(global);
    let connection = crate::common::create_connection(global, |builder|
    {
        builder.connection_retry(eventstore::Retry::Only(0))
    })?;

    check_single_node_connection(global, tcp_port, connection)

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

