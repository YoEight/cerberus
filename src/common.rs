use eventstore::ConnectionBuilder;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

#[derive(Debug, Copy, Clone)]
pub struct User<'a> {
    pub login: &'a str,
    pub password: Option<&'a str>,
}

impl<'a> User<'a> {
    pub fn from_args(global: &'a clap::ArgMatches<'_>) -> Option<User<'a>> {
        global.value_of("login").map(|login| {
            let password = global.value_of("password");

            User { login, password }
        })
    }

    pub fn to_credentials(&'a self) -> eventstore::Credentials {
        eventstore::Credentials::new(
            self.login.to_owned(),
            self.password.unwrap_or("").to_owned(),
        )
    }
}

#[derive(Serialize, Deserialize)]
pub struct NodeInfo {
    #[serde(rename = "esVersion")]
    pub version: String,

    pub state: String,

    #[serde(rename = "projectionsMode")]
    pub projections_mode: String,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct CroppedProjectionInfo {
    pub status: String,

    /// In case of a 'Faulted' status, gives an insight of what
    /// wrong happened.
    #[serde(rename = "stateReason")]
    pub reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClusterMembers {
    pub members: Vec<ClusterMember>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClusterMember {
    #[serde(rename = "externalTcpIp")]
    pub external_tcp_ip: String,

    #[serde(rename = "externalHttpIp")]
    pub external_http_ip: String,

    #[serde(rename = "externalTcpPort")]
    pub external_tcp_port: u16,

    #[serde(rename = "externalHttpPort")]
    pub external_http_port: u16,

    #[serde(rename = "internalTcpPort")]
    pub internal_tcp_port: u16,

    #[serde(rename = "internalHttpPort")]
    pub internal_http_port: u16,

    pub state: String,

    #[serde(rename = "isAlive")]
    pub is_alive: bool,
}

#[derive(Debug)]
pub enum CerberusError {
    UserFault(String),
    DevFault(String),
}

impl CerberusError {
    pub fn user_fault<S: AsRef<str>>(msg: S) -> Box<dyn Error> {
        Box::new(CerberusError::UserFault(msg.as_ref().to_owned()))
    }

    pub fn dev_fault<S: AsRef<str>>(msg: S) -> Box<dyn Error> {
        Box::new(CerberusError::DevFault(msg.as_ref().to_owned()))
    }

    pub fn boxed(self) -> Box<dyn Error> {
        Box::new(self)
    }
}

impl Error for CerberusError {}

impl fmt::Display for CerberusError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CerberusError::UserFault(msg) => write!(f, "{}", msg),

            CerberusError::DevFault(msg) => {
                writeln!(
                    f,
                    "You encountered an application unexpected error. Please \
                    report an issue there https://github.com/YoEight/cerberus/issues/new:"
                )?;

                write!(f, "Unexpected error >>= {}", msg)
            }
        }
    }
}

impl std::convert::From<std::io::Error> for CerberusError {
    fn from(source: std::io::Error) -> Self {
        CerberusError::UserFault(format!("{}", source))
    }
}

impl std::convert::From<toml::de::Error> for CerberusError {
    fn from(source: toml::de::Error) -> Self {
        CerberusError::UserFault(format!("TOML parsing error: {}", source))
    }
}

pub type CerberusResult<A> = Result<A, Box<dyn Error>>;

pub fn list_tcp_endpoints(params: &clap::ArgMatches) -> CerberusResult<Vec<SocketAddr>> {
    let mut hosts = params
        .values_of_lossy("host")
        .unwrap_or_else(|| vec!["localhost".to_owned()]);

    let port = params.value_of("tcp-port").unwrap_or("1113");
    let host = hosts.pop().unwrap();

    let endpoints: Vec<SocketAddr> = format!("{}:{}", host, port).to_socket_addrs()?.collect();

    if endpoints.is_empty() {
        Err(CerberusError::user_fault(format!(
            "Failed to produce an endpoint from [{}:{}]",
            host, port
        )))
    } else {
        Ok(endpoints)
    }
}

pub fn list_hosts<'a>(params: &'a clap::ArgMatches) -> Vec<&'a str> {
    if let Some(hosts) = params.values_of("host") {
        hosts.collect()
    } else {
        vec!["localhost"]
    }
}

/// TODO - Support cluster connection.
pub async fn create_connection<F>(
    params: &clap::ArgMatches<'_>,
    make: F,
) -> CerberusResult<eventstore::Connection>
where
    F: FnOnce(ConnectionBuilder) -> ConnectionBuilder,
{
    let mut endpoints = list_tcp_endpoints(params)?;

    let credentials_opt = if let Some(login) = params.value_of("login") {
        let password = params.value_of("password").unwrap_or("");

        Some(eventstore::Credentials::new(
            login.to_owned(),
            password.to_owned(),
        ))
    } else {
        None
    };

    let mut builder = eventstore::Connection::builder();

    if let Some(creds) = credentials_opt {
        builder = builder.with_default_user(creds);
    }

    if let Some(delay) = params.value_of("tcp-heartbeat-delay") {
        let delay = delay.parse().map_err(|e| {
            CerberusError::UserFault(format!("Failed to parse --tcp-heartbeat-delay: {}", e))
        })?;

        builder = builder.heartbeat_delay(Duration::from_millis(delay))
    }

    if let Some(timeout) = params.value_of("tcp-heartbeat-timeout") {
        let timeout = timeout.parse().map_err(|e| {
            CerberusError::UserFault(format!("Failed to parse --tcp-heartbeat-timeout: {}", e))
        })?;

        builder = builder.heartbeat_timeout(Duration::from_millis(timeout))
    }

    if let Some(count) = params.value_of("tcp-retry-count") {
        let count = count.parse().map_err(|e| {
            CerberusError::UserFault(format!("Failed to parse --tcp-retry-count: {}", e))
        })?;

        builder = builder.connection_retry(eventstore::Retry::Only(count))
    }

    if let Some(count) = params.value_of("tcp-operation-retry-count") {
        let count = count.parse().map_err(|e| {
            CerberusError::UserFault(format!(
                "Failed to parse --tcp-operation-retry-count: {}",
                e
            ))
        })?;

        builder = builder.operation_retry(eventstore::Retry::Only(count))
    }

    builder = make(builder);

    let endpoint = endpoints
        .pop()
        .expect("We already checked that list was non-empty");

    Ok(builder.single_node_connection(endpoint).await)
}

pub async fn create_connection_default(
    params: &clap::ArgMatches<'_>,
) -> CerberusResult<eventstore::Connection> {
    create_connection(params, |b| b).await
}

pub fn node_host<'a>(global: &'a clap::ArgMatches) -> &'a str {
    if let Some(mut hosts) = global.values_of("host") {
        hosts.next().unwrap_or("localhost")
    } else {
        "localhost"
    }
}

pub fn public_tcp_port(global: &clap::ArgMatches) -> u16 {
    let mut ports = global
        .values_of_lossy("tcp-port")
        .unwrap_or_else(|| vec!["1113".to_owned()]);

    ports.pop().unwrap().parse().unwrap()
}

pub fn public_http_port(global: &clap::ArgMatches) -> u16 {
    let mut ports = global
        .values_of_lossy("http-port")
        .unwrap_or_else(|| vec!["2113".to_owned()]);

    ports.pop().unwrap().parse().unwrap()
}
