mod command;
mod common;

use clap::{Arg, App, SubCommand};

fn main()
{
    let matches = App::new("Cerberus - An EventStore administration tool")
        .author("Yorick Laupa <yo.eight@gmail.com>")
        .about("An EventStore administration tool")
        .arg(Arg::with_name("host")
            .short("h")
            .long("host")
            .value_name("HOST")
            .help("A node host [default: localhost]")
            .takes_value(true)
            .multiple(true))
        .arg(Arg::with_name("tcp-port")
            .long("tcp-port")
            .help("A node port [default: 1113]")
            .takes_value(true))
        .subcommand(SubCommand::with_name("check")
            .about("Check if a database setup is reachable")
            .help("Check if a database setup is reachable"))
        .subcommand(SubCommand::with_name("list-events")
            .about("List a stream's events")
            .help("List a stream's events"))
        .get_matches();

    let result = {
        if let Some(params) = matches.subcommand_matches("check") {
            command::check::run(&matches, params)
        } else if let Some(_params) = matches.subcommand_matches("list-events") {
            unimplemented!()
        } else {
            Ok(())
        }
    };

    match result {
        Err(e) =>  {
            eprintln!("{}", e);

            std::process::exit(1);
        },

        _ => std::process::exit(0),
    }
}
