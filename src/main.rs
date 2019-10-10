mod command;
mod common;

use clap::{ Arg, App, SubCommand };

fn main()
{
    let matches = App::new("Cerberus")
        .version("1.0")
        .about("An EventStore administration tool.")
        .author("Yorick L. <yo.eight@gmail.com>")
        .arg(Arg::with_name("login")
            .help("Your user's login")
            .short("l")
            .value_name("LOGIN")
            .long("login")
            .takes_value(true))
        .arg(Arg::with_name("password")
            .help("Your user's password")
            .value_name("PASSWORD")
            .long("password")
            .takes_value(true))
        .arg(Arg::with_name("host")
            .help("A node host [default: localhost]")
            .value_name("HOST")
            .short("h")
            .long("host")
            .takes_value(true)
            .multiple(true))
        .arg(Arg::with_name("tcp-port")
            .help("A node port [default: 1113]")
            .value_name("PORT")
            .long("tcp-port")
            .takes_value(true))
        .subcommand(SubCommand::with_name("check")
            .about("Check if a database setup is reachable"))
        .subcommand(SubCommand::with_name("list")
            .about("List database entities")
            .arg(Arg::with_name("ENTITY")
                .required(true)
                .help("Database entity [events, subscriptions,…etc]"))
            .arg(Arg::with_name("stream")
                .help("A stream's name")
                .short("s")
                .long("stream")
                .value_name("STREAM_NAME")
                .takes_value(true))
            .arg(Arg::with_name("group-id")
                .help("Persistent subscription's group id")
                .long("group-id")
                .value_name("GROUP_ID")
                .takes_value(true))
            .arg(Arg::with_name("category")
                .help("Listing by category of streams")
                .long("by-category")
                .value_name("CATEGORY")
                .takes_value(true)))
        .get_matches();

    let result = {
        if let Some(params) = matches.subcommand_matches("check") {
            command::check::run(&matches, params)
        } else if let Some(params) = matches.subcommand_matches("list") {
            let entity = params.value_of("ENTITY")
                .expect("Already checked by Clap that entity isn't empty");

            match entity {
                "events" => {
                    command::list::events::run(&matches, params)
                },

                "streams" => {
                    command::list::streams::run(&matches, params)
                },


                ignored =>
                    Err(
                        common::CerberusError::UserFault(
                            format!("Listing [{}] entity is not supported yet", ignored))),
            }
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
