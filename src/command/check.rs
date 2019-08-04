use crate::args;
use crate::common::CerberusError;
use futures::future::Future;

pub fn run(params: args::Args) -> Result<(), CerberusError> {
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
            let error = CerberusError::ConnectionError(params.host, params.port);

            return Err(error);
        }
    }

    println!(
        "Successfully connected to node {}:{} through its public TCP port.",
        params.host, params.port);

    Ok(())
}
