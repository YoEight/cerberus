use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "cerberus", about = "An EventStore administration tool", author = "Yorick Laupa <yo.eight@gmail.com>")]
struct Args {
    #[structopt(long = "host", short = "h", default_value = "127.0.0.1", help = "Database host.")]
    host: String,

    #[structopt(long = "port", short = "p", default_value = "1113", help = "Database TCP port.")]
    port: u16,

    #[structopt(long = "login", help = "User login.")]
    login: Option<String>,

    #[structopt(long = "password", help = "User password.")]
    password: Option<String>,
}

fn main() {
    let args = Args::from_args();

    println!("{:?}", args);
}
