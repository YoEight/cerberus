#[macro_use]
extern crate clap;

mod command;
mod common;

fn main()
{
    let matches = clap_app!(cerberus =>
        (version: "1.0")
        (author: "Yorick L. <yo.eight@gmail.com>")
        (about: "An EventStore administration tool")
        (@arg host: -h --host +takes_value +multiple "A node host [default: localhost]")
        (@arg tcp_port: --tcp_port +takes_value +multiple "A node port [default: 1113]")
        (@arg login: -l --login +takes_value "Your user's login")
        (@arg password: --password +takes_value "Your user's password")
        (@subcommand check =>
            (about: "Check if a database setup is reachable")
        )
        (@subcommand list =>
            (about: "List database entities")
            (@arg ENTITY: +required "Database entity [events, subscriptions,…etc]")
            (@arg stream: -s --stream +takes_value "A stream's name")
            (@arg group_id: --group_id "Persistent subscription's group id")
        )
    ).get_matches();

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
