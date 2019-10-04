pub mod events {
    use crate::common::{ CerberusResult, CerberusError };
    use futures::future::Future;
    use futures::stream::Stream;

    pub fn run(global: &clap::ArgMatches, params: &clap::ArgMatches)
        -> CerberusResult<()>
    {
        let connection = crate::common::create_connection_default(global)?;

        if let Some(stream_name) = params.value_of("stream") {
            let stream = connection
                .read_stream(stream_name)
                .resolve_link_tos(eventstore::LinkTos::ResolveLink)
                .iterate_over();

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
