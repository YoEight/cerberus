use crate::args;

use futures::future::Future;

pub fn run(params: args::Args) {
    let addr = params.parse_tcp_socket_addr();
    let connection = eventstore::Connection::builder()
        .connection_retry(eventstore::Retry::Only(0))
        .single_node_connection(addr);

    let result = connection.read_all()
        .start_from_beginning()
        .max_count(1)
        .execute()
        .wait();

    if let Err(e) = result {
        if let eventstore::OperationError::Aborted = e {
            eprintln!(
                "Failed to connect to node {}:{} through its public TCP port.",
                params.host, params.port);

            return;
        }
    }

    println!(
        "Successfully connected to node {}:{} through its public TCP port.",
        params.host, params.port);
}
