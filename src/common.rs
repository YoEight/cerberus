use eventstore::ConnectionBuilder;
use std::error::Error;
use std::fmt;
use std::net::{ SocketAddr, ToSocketAddrs };

#[derive(Debug)]
pub enum CerberusError {
    UserFault(String),
}

impl Error for CerberusError {}

impl fmt::Display for CerberusError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CerberusError::UserFault(msg) =>
                write!(f, "{}", msg)
        }
    }
}

pub type CerberusResult<A> = Result<A, CerberusError>;

pub fn list_tcp_endpoints(params: &clap::ArgMatches)
    -> CerberusResult<Vec<SocketAddr>>
{
    let mut hosts = params.values_of_lossy("host")
        .unwrap_or(vec!["localhost".to_owned()]);

    let port = params.value_of("tcp-port").unwrap_or("1113");
    let host = hosts.pop().unwrap();

    match format!("{}:{}", host, port).to_socket_addrs() {
        Err(e) =>
            Err(
                CerberusError::UserFault(
                    format!("Failed to resolve [{}:{}]: {}", host, port, e))),

        Ok(sock_iter) => {
            let endpoints: Vec<SocketAddr> = sock_iter.collect();

            if endpoints.is_empty() {
                Err(
                    CerberusError::UserFault(
                        format!("Failed to produce an endpoint from [{}:{}]", host, port)))
            } else {
                Ok(endpoints)
            }
        },
    }
}

/// TODO - Support cluster connection.
pub fn create_connection<F>(params: &clap::ArgMatches, make: F)
    -> CerberusResult<eventstore::Connection>
    where
        F: FnOnce(ConnectionBuilder) -> ConnectionBuilder
{
    let mut endpoints = list_tcp_endpoints(params)?;

    let credentials_opt =
        if let Some(login) = params.value_of("login") {
            let password = params.value_of("password").unwrap_or("");

            Some(eventstore::Credentials::new(login, password))
        } else {
            None
        };

    let mut builder = eventstore::Connection::builder();

    if let Some(creds) = credentials_opt {
        builder = builder.with_default_user(creds);
    }

    builder = make(builder);

    let endpoint = endpoints.pop().expect("We already checked that list was non-empty");

    Ok(builder.single_node_connection(endpoint))
}

pub fn create_connection_default(params: &clap::ArgMatches)
    -> CerberusResult<eventstore::Connection>
{
    create_connection(params, |b| b)
}
