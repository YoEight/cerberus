mod args;
mod command;
mod common;

fn main() -> Result<(), common::CerberusError> {
    let params = args::Args::parse_from_cli();

    let result = match params.cmd {
        args::Cmd::Check =>
            command::check::run(params),
    };

    match result {
        Err(e) =>  {
            println!("{}", e);

            std::process::exit(1);
        },

        _ => std::process::exit(0),
    }
}
