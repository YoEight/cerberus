use std::net::SocketAddr;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "cerberus", about = "An EventStore administration tool", author = "Yorick Laupa <yo.eight@gmail.com>")]
pub struct Args {
    #[structopt(long = "host", short = "h", default_value = "127.0.0.1", help = "Database host.")]
    pub host: String,

    #[structopt(long = "tcp-port", short = "p", default_value = "1113", help = "Database TCP port.")]
    pub port: u16,

    #[structopt(long = "http-port", default_value = "2113", help = "Database public HTTP port.")]
    pub http_port: u16,

    #[structopt(long = "login", help = "User login.")]
    pub login: Option<String>,

    #[structopt(long = "password", help = "User password.")]
    pub password: Option<String>,

    #[structopt(subcommand)]
    pub cmd: Cmd,
}

#[derive(Debug, StructOpt)]
pub enum Cmd {
    #[structopt(name = "check", help = "Checks if a database node is reachable.", about = "Checks if a database node is reachable.")]
    Check,
}

impl Args {
    pub fn parse_from_cli() -> Args {
        Args::from_args()
    }

    pub fn parse_tcp_socket_addr(&self) -> SocketAddr {
        Args::parse_socket_addr(self.host.clone(), self.port)
    }

    fn parse_socket_addr(host: String, port: u16) -> SocketAddr {
        format!("{}:{}", host, port).parse().unwrap()
    }
}
