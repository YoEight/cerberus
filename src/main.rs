#[macro_use]
extern crate log;

#[macro_use]
extern crate clap;

mod api;
mod command;
mod common;

use clap::{App, Arg, SubCommand};
use std::env;

#[tokio::main]
async fn main() {
    let matches = App::new("Cerberus")
        .version(crate_version!())
        .about(crate_description!())
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
            .help("A node TCP port [default: 1113]")
            .value_name("PORT")
            .long("tcp-port")
            .takes_value(true))
        .arg(Arg::with_name("http-port")
            .help("A node HTTP port [default: 2113]")
            .value_name("PORT")
            .long("http-port")
            .takes_value(true)
            .multiple(true))
        .arg(Arg::with_name("tcp-heartbeat-delay")
            .help("Delay without receiving any news from the server so we start to worry [default: 750ms]")
            .long("tcp-heartbeat-delay")
            .takes_value(true))
        .arg(Arg::with_name("tcp-heartbeat-timeout")
            .help(
                "After --tcp-heartbeat-delay, the delay after a healthcheck goes unanswered, \
                leading to close the connection (can be retried) [default: 1500ms]")
            .long("tcp-heartbeat-timeout")
            .takes_value(true))
        .arg(Arg::with_name("tcp-retry-count")
            .help("Max connection retry count in case of connection error [default: 3]")
            .long("tcp-retry-count")
            .takes_value(true))
        .arg(Arg::with_name("tcp-operation-retry-count")
            .help("Max operation retry count in case of no response from the server [default: 3]")
            .long("tcp-operation-retry-count")
            .takes_value(true))
        .arg(Arg::with_name("verbose")
            .help("Log verbosity. The more -v there are, the more verbose it gets")
            .short("v")
            .multiple(true))
        .subcommand(SubCommand::with_name("check")
            .about("Check if a database setup is reachable")
            .arg(Arg::with_name("no-cluster-check")
                .help("Discard cluster connection health-check")
                .long("no-cluster-check")))
        .subcommand(SubCommand::with_name("list-events")
            .about("List stream events")
            .arg(Arg::with_name("stream")
                .help("For events and subscription entities, represents a stream's name")
                .short("s")
                .long("stream")
                .value_name("STREAM_NAME")
                .takes_value(true))
            .arg(Arg::with_name("group-id")
                .help(
                    "Represents a persistent subscription's group-id. Can be used with \
                    --stream parameter to get a persistent subscription parked messages \
                    events")
                .long("group-id")
                .value_name("GROUP_ID")
                .takes_value(true))
            .arg(Arg::with_name("checkpoint")
                .help(
                    "When --stream and --group-id are used, will give a persistent \
                    subscription's last persisted checkpoint")
                .long("checkpoint"))
            .arg(Arg::with_name("by-type")
                .help("Targets events of a given type")
                .long("by-type")
                .value_name("Type")
                .takes_value(true))
            .arg(Arg::with_name("recent")
                .help("For streams and events entities, takes the recent 50 entries")
                .long("recent")))
        .subcommand(SubCommand::with_name("list-streams")
            .about(
                "List streams. Don't expect to see internal streams except \
                metadata and deletion stream")
            .arg(Arg::with_name("category")
                .help(
                    "List streams by a given category. For example, if you pass \"foo\"
                    as a category. The command will return streams starting
                    with \"foo-\"")
                .long("by-category")
                .value_name("CATEGORY")
                .takes_value(true))
            .arg(Arg::with_name("recent")
                .help("For streams and events entities, takes the recent 50 entries")
                .long("recent")))
        .subcommand(SubCommand::with_name("list-subscription")
            .about("List a persistent subscription information")
            .arg(Arg::with_name("stream")
                .help("For events and subscription entities, represents a stream's name")
                .short("s")
                .long("stream")
                .value_name("STREAM_NAME")
                .takes_value(true)
                .required(true))
            .arg(Arg::with_name("group-id")
                .help("A persistent subscription's group-id")
                .long("group-id")
                .value_name("GROUP_ID")
                .takes_value(true)
                .required(true)))
        .subcommand(SubCommand::with_name("list-subscriptions")
            .about("List persistent subscriptions")
            .arg(Arg::with_name("raw")
                .help("Displays the persistent subscriptions as-is from the server")
                .long("raw")))
        .subcommand(SubCommand::with_name("create-subscription")
            .about("Create a persistent subscription")
            .arg(Arg::with_name("stream")
                .help("Stream's name")
                .long("stream")
                .short("s")
                .value_name("STREAM_NAME")
                .required(true)
                .takes_value(true))
            .arg(Arg::with_name("group-id")
                .help("Persistent subscription's group id")
                .long("group-id")
                .short("g")
                .value_name("GROUP_ID")
                .required(true)
                .takes_value(true))
            .arg(Arg::with_name("resolve-link")
                .help("Determines whether any link events encountered in the stream will be resolved")
                .long("resolve-link"))
            .arg(Arg::with_name("start-from")
                .help("Where the subscription should start from (event number) [default: -1]")
                .long("start-from")
                .value_name("EVENT_NUMBER")
                .takes_value(true))
            .arg(Arg::with_name("extra-stats")
                .help("Whether or not in depth latency statistics should be tracked on this subscription")
                .long("extra-stats"))
            .arg(Arg::with_name("message-timeout")
                .help(
                    "The amount of time, in milliseconds, after which a message \
                    should be considered to be timeout and retried [default: 30secs]")
                .value_name("MILLISECONDS")
                .long("message-timeout")
                .takes_value(true))
            .arg(Arg::with_name("max-retry-count")
                .help(
                    "The maximum number of retries (due to timeout) before a message \
                    get considered to be parked [default: 10]")
                .long("max-retry-count")
                .value_name("COUNT")
                .takes_value(true))
            .arg(Arg::with_name("live-buffer-size")
                .help("The size of the buffer listening to live messages as they happen [default: 500]")
                .long("live-buffer-size")
                .value_name("BUFFER_SIZE")
                .takes_value(true))
            .arg(Arg::with_name("read-batch-size")
                .help("The number of events read at a time when paging in history [default: 500]")
                .long("read-batch-size")
                .value_name("BUFFER_SIZE")
                .takes_value(true))
            .arg(Arg::with_name("history-buffer-size")
                .help("The number of events read at a time when paging through history [default: 500]")
                .long("history-buffer-size")
                .value_name("BUFFER_SIZE")
                .takes_value(true))
            .arg(Arg::with_name("checkpoint-after")
                .help("The amount of time, in milliseconds, to try checkpoint after [default: 2secs]")
                .long("checkpoint-after")
                .value_name("MILLISECONDS")
                .takes_value(true))
            .arg(Arg::with_name("min-checkpoint-count")
                .help("The minimum number of messages to checkpoint [default: 10]")
                .long("min-checkpoint-count")
                .value_name("COUNT")
                .takes_value(true))
            .arg(Arg::with_name("max-checkpoint-count")
                .help(
                    "The maximum number of messages to checkpoint. \
                    If this number is reached , a checkpoint will be forced [default: 1000]")
                .long("max-checkpoint-count")
                .value_name("COUNT")
                .takes_value(true))
            .arg(Arg::with_name("max-subs-count")
                .help("The maximum number of subscribers allowed [default: 0 (means no limit)]")
                .long("max-subs-count")
                .value_name("COUNT")
                .takes_value(true))
            .arg(Arg::with_name("consumer-strategy")
                .help("The strategy to use for distributing events to client consumers [default: RoundRobin]")
                .long("consumer-strategy")
                .value_name("STRATEGY")
                .takes_value(true)))
        .subcommand(SubCommand::with_name("update-subscription")
            .about("Update a persistent subscription")
            .arg(Arg::with_name("stream")
                .help("Stream's name")
                .long("stream")
                .short("s")
                .value_name("STREAM_NAME")
                .required(true)
                .takes_value(true))
            .arg(Arg::with_name("group-id")
                .help("Persistent subscription's group id")
                .long("group-id")
                .short("g")
                .value_name("GROUP_ID")
                .required(true)
                .takes_value(true))
            .arg(Arg::with_name("resolve-link")
                .help("Determines whether any link events encountered in the stream will be resolved")
                .long("resolve-link"))
            .arg(Arg::with_name("start-from")
                .help("Where the subscription should start from (event number) [default: -1]")
                .long("start-from")
                .value_name("EVENT_NUMBER")
                .takes_value(true))
            .arg(Arg::with_name("extra-stats")
                .help("Whether or not in depth latency statistics should be tracked on this subscription")
                .long("extra-stats"))
            .arg(Arg::with_name("message-timeout")
                .help(
                    "The amount of time, in milliseconds, after which a message \
                    should be considered to be timeout and retried [default: 30secs]")
                .value_name("MILLISECONDS")
                .long("message-timeout")
                .takes_value(true))
            .arg(Arg::with_name("max-retry-count")
                .help(
                    "The maximum number of retries (due to timeout) before a message \
                    get considered to be parked [default: 10]")
                .long("max-retry-count")
                .value_name("COUNT")
                .takes_value(true))
            .arg(Arg::with_name("live-buffer-size")
                .help("The size of the buffer listening to live messages as they happen [default: 500]")
                .long("live-buffer-size")
                .value_name("BUFFER_SIZE")
                .takes_value(true))
            .arg(Arg::with_name("read-batch-size")
                .help("The number of events read at a time when paging in history [default: 500]")
                .long("read-batch-size")
                .value_name("BUFFER_SIZE")
                .takes_value(true))
            .arg(Arg::with_name("history-buffer-size")
                .help("The number of events read at a time when paging through history [default: 500]")
                .long("history-buffer-size")
                .value_name("BUFFER_SIZE")
                .takes_value(true))
            .arg(Arg::with_name("checkpoint-after")
                .help("The amount of time, in milliseconds, to try checkpoint after [default: 2secs]")
                .long("checkpoint-after")
                .value_name("MILLISECONDS")
                .takes_value(true))
            .arg(Arg::with_name("min-checkpoint-count")
                .help("The minimum number of messages to checkpoint [default: 10]")
                .long("min-checkpoint-count")
                .value_name("COUNT")
                .takes_value(true))
            .arg(Arg::with_name("max-checkpoint-count")
                .help(
                    "The maximum number of messages to checkpoint. \
                    If this number is reached , a checkpoint will be forced [default: 1000]")
                .long("max-checkpoint-count")
                .value_name("COUNT")
                .takes_value(true))
            .arg(Arg::with_name("max-subs-count")
                .help("The maximum number of subscribers allowed [default: 0 (means no limit)]")
                .long("max-subs-count")
                .value_name("COUNT")
                .takes_value(true))
            .arg(Arg::with_name("consumer-strategy")
                .help("The strategy to use for distributing events to client consumers [default: RoundRobin]")
                .long("consumer-strategy")
                .value_name("STRATEGY")
                .takes_value(true)))
        .subcommand(SubCommand::with_name("delete-subscription")
            .about("Delete a persistent subscription")
            .arg(Arg::with_name("stream")
                .help("A stream's name")
                .short("s")
                .long("stream")
                .takes_value(true)
                .required(true)
                .value_name("STREAM_NAME"))
            .arg(Arg::with_name("group-id")
                .help("Persistent subscription's group id")
                .short("g")
                .long("group-id")
                .takes_value(true)
                .required(true)
                .value_name("GROUP_ID"))
            .arg(Arg::with_name("confirm")
                .long("confirm")
                .required(true)))
        .subcommand(SubCommand::with_name("create-projection")
            .about("Create a projection")
            .arg(Arg::with_name("name")
                .help("Projection's name")
                .short("n")
                .long("name")
                .takes_value(true)
                .value_name("NAME"))
            .arg(Arg::with_name("kind")
                .help("Kind of the projection [onetime, transient, continuous]")
                .short("k")
                .long("kind")
                .required(true)
                .takes_value(true)
                .value_name("TYPE"))
            .arg(Arg::with_name("enabled")
                .help("Indicates if the projection is enabled as soon as transmitted to the server")
                .long("enabled"))
            .arg(Arg::with_name("emit")
                .help("Enable the ability for the projection to write to streams")
                .long("emit"))
            .arg(Arg::with_name("checkpoints")
                .help("Enable checkpoints. Think saving progression, like in video games")
                .long("checkpoint"))
            .arg(Arg::with_name("track-emitted-streams")
                .help("Write the name of the streams the projection is managing to a separate stream")
                .long("track-emitted-streams"))
            .arg(Arg::with_name("SCRIPT")
                .help("Path to the projection's Javascript script")
                .required(true)
                .value_name("FILEPATH")))
        .subcommand(SubCommand::with_name("list-projections")
            .about("List all projections")
            .arg(Arg::with_name("kind")
                .help("Kind of projection [onetime, transient, continuous, all-non-transient, any: default any]")
                .value_name("KIND")
                .short("k")
                .long("kind")
                .takes_value(true)))
        .subcommand(SubCommand::with_name("export")
            .about("Export events from a database to another, that command connects to source database through its TCP port")
            .arg(Arg::with_name("from-stream")
                .help("Stream to copy events from")
                .long("from-stream")
                .value_name("STREAM_NAME")
                .takes_value(true))
            .arg(Arg::with_name("from-type")
                .help("Copy events of a given type. Those events will be written to the same stream those belong from the source")
                .long("from-type")
                .value_name("TYPE")
                .takes_value(true))
            .arg(Arg::with_name("from-category")
                .help("Copy events from streams that belong to given category")
                .long("from-category")
                .value_name("CATEGORY")
                .takes_value(true))
            .arg(Arg::with_name("to-host")
                .help("Host of the destination database")
                .long("to-host")
                .value_name("HOST")
                .takes_value(true)
                .required(true))
            .arg(Arg::with_name("to-tcp-port")
                .help("TCP port of the destination database [default: 1113]")
                .long("to-tcp-port")
                .value_name("PORT")
                .takes_value(true))
            .arg(Arg::with_name("recent")
                .help("Takes only the latest 50 entries")
                .long("recent"))
            .arg(Arg::with_name("top")
                .help("Takes only the latest N entries")
                .long("top")
                .takes_value(true)))
        .subcommand(SubCommand::with_name("backup")
            .about("Backup an eventstore node's data")
            .arg(Arg::with_name("source-directory")
                .help("Source directory where the backup will be created from")
                .long("source-directory")
                .takes_value(true)
                .required(true))
            .arg(Arg::with_name("destination-directory")
                .help("Destination directory where the backup will be stored")
                .long("destination-directory")
                .takes_value(true)
                .required(true))
            .arg(Arg::with_name("remote-user")
                .help("User to use on remote server")
                .long("remote-user")
                .takes_value(true)))
        .subcommand(SubCommand::with_name("apply-compliance")
            .about("Verifies the database meets all the requirements specified by a compliance file (TOML format)")
            .arg(Arg::with_name("file")
                .help("Compliance filepath. It requires a TOML file")
                .long("file")
                .takes_value(true)
                .required(true))
            .arg(Arg::with_name("dry-run")
                .help("Run the compliance checking process without persisting anything")
                .long("dry-run")))
        .get_matches();

    let verbosity = matches.occurrences_of("verbose");
    let user_opt = common::User::from_args(&matches);
    let host = crate::common::node_host(&matches);
    let http_port = crate::common::public_http_port(&matches);
    let api = api::Api::new(host, http_port, user_opt);

    if verbosity > 0 {
        match verbosity {
            1 => env::set_var("RUST_LOG", "cerberus=info"),

            2 => env::set_var("RUST_LOG", "cerberus=info,eventstore=info"),

            _ => env::set_var("RUST_LOG", "cerberus=debug,eventstore=debug"),
        }

        env_logger::init();
    }

    let result = {
        if let Some(params) = matches.subcommand_matches("check") {
            command::check::run(&matches, params, api).await
        } else if let Some(params) = matches.subcommand_matches("list-streams") {
            command::list::streams::run(&matches, params).await
        } else if let Some(params) = matches.subcommand_matches("list-events") {
            command::list::events::run(&matches, params).await
        } else if let Some(params) = matches.subcommand_matches("list-subscriptions") {
            command::list::subscriptions::run(&matches, params, api).await
        } else if let Some(params) = matches.subcommand_matches("list-subscription") {
            command::list::subscription::run(&matches, params, api).await
        } else if let Some(params) = matches.subcommand_matches("create-subscription") {
            command::create::subscription::run(&matches, params, user_opt).await
        } else if let Some(params) = matches.subcommand_matches("update-subscription") {
            command::update::subscription::run(&matches, params, user_opt).await
        } else if let Some(params) = matches.subcommand_matches("delete-subscription") {
            command::delete::subscription::run(&matches, params, user_opt).await
        } else if let Some(params) = matches.subcommand_matches("create-projection") {
            command::create::projection::run(&matches, params, api).await
        } else if let Some(params) = matches.subcommand_matches("list-projections") {
            command::list::projections::run(&matches, params, api).await
        } else if let Some(params) = matches.subcommand_matches("export") {
            command::export::run(&matches, params).await
        } else if let Some(params) = matches.subcommand_matches("backup") {
            command::backup::run(&matches, params)
        } else if let Some(params) = matches.subcommand_matches("apply-compliance") {
            command::compliance::run(&matches, params, api).await
        } else {
            Ok(())
        }
    };

    match result {
        Err(e) => {
            eprintln!("{}", e);

            std::process::exit(1);
        }

        _ => std::process::exit(0),
    }
}
