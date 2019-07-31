use futures::future::Future;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "cerberus", about = "An EventStore administration tool", author = "Yorick Laupa <yo.eight@gmail.com>")]
struct Args {
    #[structopt(long = "host", short = "h", default_value = "127.0.0.1", help = "Database host.")]
    host: String,

    #[structopt(long = "tcp-port", short = "p", default_value = "1113", help = "Database TCP port.")]
    port: u16,

    #[structopt(long = "http-port", default_value = "2113", help = "Database public HTTP port.")]
    http_port: u16,

    #[structopt(long = "login", help = "User login.")]
    login: Option<String>,

    #[structopt(long = "password", help = "User password.")]
    password: Option<String>,

    #[structopt(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, StructOpt)]
enum Cmd {
    #[structopt(name = "check", help = "Checks if a database node is reachable.", about = "Checks if a database node is reachable.")]
    Check,
}

fn main() {
    let args = Args::from_args();

    match args.cmd {
        Cmd::Check => {
            let addr = format!("{}:{}", args.host, args.port).parse().unwrap();
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
                        args.host, args.port);

                    return;
                }
            }

            println!(
                "Successfully connected to node {}:{} through its public TCP port.",
                args.host, args.port);
        }
    }
}
