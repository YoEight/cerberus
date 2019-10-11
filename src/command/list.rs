pub mod events {
    use crate::common::{ CerberusResult, CerberusError };
    use eventstore::{ ResolvedEvent, OperationError };
    use futures::future::Future;
    use futures::stream::Stream;

    fn get_stream_name(original_stream_name: &str, params: &clap::ArgMatches) -> String
    {
        if let Some(group_id) = params.value_of("group-id") {
            if params.is_present("checkpoint") {
                format!("$persistentsubscription-{}::{}-checkpoint", original_stream_name, group_id)
            } else {
                format!("$persistentsubscription-{}::{}-parked", original_stream_name, group_id)
            }
        } else {
            original_stream_name.to_owned()
        }
    }

    pub fn run(global: &clap::ArgMatches, params: &clap::ArgMatches)
        -> CerberusResult<()>
    {
        let connection = crate::common::create_connection_default(global)?;

        if let Some(original_stream_name) = params.value_of("stream") {
            let command = connection
                .read_stream(get_stream_name(original_stream_name, params))
                .resolve_link_tos(eventstore::LinkTos::ResolveLink);

            let stream: Box<dyn Stream<Item=ResolvedEvent, Error=OperationError>> =
                if params.is_present("recent") {
                    // We also force `max_count` to 50 because there is no need
                    // asking the server to load more than that.
                    let stream = command
                        .max_count(50)
                        .start_from_end_of_stream()
                        .iterate_over()
                        .take(50);

                    Box::new(stream)
                } else {
                    Box::new(command.iterate_over())
                };

            let result = stream.for_each(|event|
            {
                let record = event.event.expect("Event field would be always defined in this situation");

                println!("--------------------------------------------------------------");
                println!("Number: {}", record.event_number);
                println!("Stream: {}", record.event_stream_id);
                println!("Type: {}", record.event_type);
                println!("Id: {}", record.event_id);

                if record.is_json {
                    let result =
                        record.as_json::<serde_json::value::Value>();

                    println!("Payload: ");

                    match result {
                        Ok(value) =>
                            if let Err(e) = serde_json::to_writer_pretty(std::io::stdout(), &value) {
                                println!("<Payload was supposed to be JSON: {}>", e);
                            } else {
                                println!();
                            },

                        Err(e) =>
                            println!("<Payload was supposed to be JSON: {}>", e),
                    }
                } else {
                    println!("Payload: <raw bytes we don't know how to deal with>");
                }


                Ok(())
            }).wait();

            if let Err(e) = result {
                Err(
                    CerberusError::UserFault(
                        format!("Exception happened when streaming the stream (huhuh): {}", e)))
            } else {
                Ok(())
            }
        } else {
            Err(
                CerberusError::UserFault(
                    "You didn't supply --stream option".to_owned()))
        }
    }
}

pub mod streams {
    use crate::common::{ CerberusResult, CerberusError };
    use eventstore::{ ResolvedEvent, OperationError };
    use futures::future::Future;
    use futures::stream::Stream;

    fn get_stream_name(params: &clap::ArgMatches) -> String {
        if let Some(category) = params.value_of("category") {
            return format!("$ce-{}", category);
        } else {
            "$streams".to_owned()
        }
    }

    pub fn run(global: &clap::ArgMatches, params: &clap::ArgMatches)
        -> CerberusResult<()>
    {
        let connection = crate::common::create_connection_default(global)?;
        let stream_name = get_stream_name(params);

        let command = connection
            .read_stream(stream_name.as_str())
            .resolve_link_tos(eventstore::LinkTos::ResolveLink);

        let stream: Box<dyn Stream<Item=ResolvedEvent, Error=OperationError>> =
            if params.is_present("recent") {
                // We also force `max_count` to 50 because there is no need
                // asking the server to load more than that.
                let stream = command
                    .max_count(50)
                    .start_from_end_of_stream()
                    .iterate_over()
                    .take(50);

                Box::new(stream)
            } else {
                Box::new(command.iterate_over())
            };

        let result = stream.fold(1usize, |pos, event|
        {
            match event.event {
                None => {
                    // It means we are in a situation where a stream got deleted.
                    let record = event
                        .link
                        .expect("Link field would be always defined in this situation");

                    unsafe {
                        let data = std::str::from_utf8_unchecked(&record.data);

                        // In this case, the data looks like the following:
                        // 0@whatever_stream_name_was
                        let (_, stream_name) = data.split_at(2);
                        println!("{}: [DELETED] {}", pos, stream_name);
                    }
                },

                Some(record) => {
                    println!("{}: {}", pos, record.event_stream_id);
                }
            }

            Ok(pos + 1)
        }).wait();

        match result {
            Err(e) =>
                if let eventstore::OperationError::AccessDenied(_) = e {
                    let msg =
                        format!(
                            "Action denied: You can't list [{}] stream with \
                            your current user credentials. It also possible you haven't \
                            enable system projections or start system projections. You can \
                            do both when starting the server or, if you already enabled projections, \
                            you can start those in the administration web page.", stream_name);

                    Err(CerberusError::UserFault(msg))
                } else {
                    Err(
                        CerberusError::UserFault(
                            format!("Exception happened when streaming the stream (huhuh): {}", e)))
                },

            Ok(pos) => {
                if pos == 0 {
                    println!("You have no user-defined streams yet");
                }

                Ok(())
            },
        }
    }
}

pub mod subscriptions {
    use crate::common::{ CerberusResult, CerberusError, User };

    pub fn run(global: &clap::ArgMatches, _: &clap::ArgMatches, user_opt: Option<User>)
        -> CerberusResult<()>
    {
        let base_url = crate::common::create_node_uri(global);

        let mut req = reqwest::Client::new()
            .get(&format!("{}/subscriptions", base_url));

        if let Some(user) = user_opt {
            req = req.basic_auth(user.login, user.password);
        }

        let mut resp = req.send().map_err(|e|
        {
            CerberusError::UserFault(
                format!("Failed to list persistent subscriptions: {}", e))
        })?;

        let subs: Vec<crate::common::SubscriptionSummary> = resp.json().map_err(|e|
        {
            CerberusError::UserFault(
                format!("Failed to deserialize SubscriptionSummary: {}", e))
        })?;

        for sub in subs {
            let process_diff = sub.last_known_event_number - sub.last_processed_event_number;

            println!("--------------------------------------------------------------");
            println!("Stream: {}", sub.event_stream_id);
            println!("Group: {}", sub.group_name);
            println!("Status: {}", sub.status);
            println!("Connections : {}", sub.connection_count);
            println!("Processed / Known: {} / {} ({})", sub.last_processed_event_number, sub.last_known_event_number, process_diff);
            println!("Processing speed : {} msgs/sec", sub.average_items_per_sec);
        }

        Ok(())
    }
}
