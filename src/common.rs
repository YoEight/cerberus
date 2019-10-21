use eventstore::ConnectionBuilder;
use serde::{ Serialize, Deserialize };
use std::error::Error;
use std::fmt;
use std::net::{ SocketAddr, ToSocketAddrs };

pub struct User<'a> {
    pub login: &'a str,
    pub password: Option<&'a str>,
}

impl<'a> User<'a> {
    pub fn from_args(global: &'a clap::ArgMatches) -> Option<User<'a>> {
        global.value_of("login").map(|login|
        {
            let password = global.value_of("password");

            User {
                login,
                password,
            }
        })
    }

    pub fn to_credentials(&self) -> eventstore::Credentials {
        eventstore::Credentials::new(self.login, self.password.unwrap_or(""))
    }
}

#[derive(Serialize, Deserialize)]
pub struct SubscriptionSummary {
    #[serde(rename = "eventStreamId")]
    pub event_stream_id: String,

    #[serde(rename = "groupName")]
    pub group_name: String,

    #[serde(rename = "status")]
    pub status: String,

    #[serde(rename = "averageItemsPerSecond")]
    pub average_items_per_sec: f64,

    #[serde(rename = "lastProcessedEventNumber")]
    pub last_processed_event_number: i64,

    #[serde(rename = "lastKnownEventNumber")]
    pub last_known_event_number: i64,

    #[serde(rename = "connectionCount")]
    pub connection_count: i64,
}

#[derive(Serialize, Deserialize)]
pub struct Projections {
    pub projections: Vec<Projection>,
}

#[derive(Serialize, Deserialize)]
pub struct Projection {
    pub name: String,
    pub mode: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectionCreationSuccess {
    #[serde(rename = "msgTypeId")]
    pub msg_type: usize,
    pub name: String,
}

#[derive(Debug)]
pub enum CerberusError {
    UserFault(String),
    DevFault(String),
}

impl Error for CerberusError {}

impl fmt::Display for CerberusError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CerberusError::UserFault(msg) =>
                write!(f, "{}", msg),

            CerberusError::DevFault(msg) => {
                writeln!(f,
                    "You encountered an application unexpected error. Please \
                    report an issue there https://github.com/YoEight/cerberus/issues/new:")?;

                write!(f, "Unexpected error >>= {}", msg)
            },
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

// TODO - Not sure what strategy we should pick in case of a cluster.
// Do we try to pick the elected node first?
pub fn create_node_uri(params: &clap::ArgMatches)
    -> String
{
    let mut hosts = params.values_of_lossy("host")
        .unwrap_or(vec!["localhost".to_owned()]);

    let mut ports = params.values_of_lossy("http-port")
        .unwrap_or(vec!["2113".to_owned()]);

    let host = hosts.pop().unwrap();
    let port = ports.pop().unwrap();

    format!("http://{}:{}", host, port)
}

pub mod http {
    use crate::common::{ CerberusError, CerberusResult };

    pub fn default_client_fault(mut resp: reqwest::Response) -> CerberusResult<()>
    {
        let msg = resp.text().unwrap_or("<unreadable text message>".to_owned());

        Err(
            CerberusError::UserFault(
                format!("User error: {}", msg)))
    }

    pub fn default_server_fault(mut resp: reqwest::Response) -> CerberusResult<()>
    {
        let msg = resp.text().unwrap_or("<unreadable text message>".to_owned());

        Err(
            CerberusError::UserFault(
                format!("Server error: [{}] {}", resp.status(), msg)))
    }

    pub fn default_unexpected(resp: reqwest::Response) -> CerberusResult<()>
    {
        Err(
            CerberusError::DevFault(
                format!("{:?}", resp)))
    }
}
