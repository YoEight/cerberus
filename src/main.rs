mod args;
mod command;

fn main() {
    let params = args::Args::parse_from_cli();

    match params.cmd {
        args::Cmd::Check =>
            command::check::run(params),
    }
}
