use crate::common::{ CerberusResult, CerberusError };
use eventstore::{ ResolvedEvent, OperationError };
use futures::future::Future;
use futures::stream::Stream;

enum Selection<'a> {
    EventType(&'a str),
    StreamCategory(&'a str),
    Stream(&'a str),
}

enum Limit {
    Top(usize),
    None,
}

fn get_export_selection<'a>(params: &'a clap::ArgMatches) -> CerberusResult<Selection<'a>> {
    if let Some(stream) = params.value_of("from-stream") {
        return Ok(Selection::Stream(stream));
    } else if let Some(tpe) = params.value_of("from-type") {
        return Ok(Selection::EventType(tpe));
    } else if let Some(category) = params.value_of("from-category") {
        return Ok(Selection::StreamCategory(category));
    }

    Err(CerberusError::UserFault(
        "No source submitted. You should at least provide \
        --from-stream, --from-type or --from-category".to_owned()))
}

fn get_stream_name(tpe: &Selection) -> String {
    match *tpe {
        Selection::EventType(tpe) => format!("$et-{}", tpe),
        Selection::StreamCategory(category) => format!("$category-{}", category),
        Selection::Stream(stream) => stream.to_owned(),
    }
}

fn get_limit(params: &clap::ArgMatches) -> CerberusResult<Limit> {
    if params.is_present("recent") {
        Ok(Limit::Top(50))
    } else if let Some(value) = params.value_of("top") {
        let value = value.parse().map_err(|e|
            CerberusError::UserFault(
                format!("Failed to parse --top number: {}", e))
        )?;

        if value == 0 {
            return Err(
                CerberusError::UserFault(
                    "--top parameter must be greater than 0".to_owned()));
        }

        Ok(Limit::Top(value))

    } else {
        Ok(Limit::None)
    }
}

const DEFAULT_BUFFER_SIZE: usize = 500;

fn record_to_event_data(record: &eventstore::RecordedEvent) -> eventstore::EventData {
    let data = if record.is_json {
        let json: &serde_json::value::RawValue = record.as_json().unwrap();

        eventstore::EventData::json(&*record.event_type, json).unwrap()
    } else {
        eventstore::EventData::binary(&*record.event_type, record.data.clone())
    };

    data.id(record.event_id)
}

fn export_by_category<S>(
    source_connection: &eventstore::Connection,
    destination_connection: &eventstore::Connection,
    stream: S,
) -> CerberusResult<()>
    where
        S: Stream<Item=ResolvedEvent, Error=OperationError>
{
    let result = stream.for_each(|event| {
        let record = event.event.expect("Event field must be defined in this case");
        let target_stream_name = unsafe {
            std::str::from_utf8_unchecked(&record.data).to_owned()
        };

        // TODO - It's possible the stream is deleted. We should skip it in
        // such a case.
        let inner_source = source_connection
            .read_stream(target_stream_name.as_ref())
            .iterate_over()
            .chunks(DEFAULT_BUFFER_SIZE);

        inner_source.for_each(move |chunk| {
            let events = chunk.into_iter().map(|event| {
                let record = event.event.expect("Targetted event must be defined");

                record_to_event_data(&record)
            });

            info!("Copy stream {} ...", target_stream_name);

            destination_connection
                .write_events(target_stream_name.as_ref())
                .append_events(events)
                .execute()
                .map(|_| ())
        })
    }).wait();

    result.map_err(|e|
        CerberusError::UserFault(
            format!("Error occured when exporting by category: {}", e))
    )
}

fn export_by_type<S>(
    destination_connection: &eventstore::Connection,
    stream: S,
) -> CerberusResult<()>
    where
        S: Stream<Item=ResolvedEvent, Error=OperationError>
{
    let result = stream.for_each(|event| {
        let record = event.event.expect("Event field must be defined in this case");

        println!(
            "Copy event {} of type {} to stream {}",
            record.event_id,
            record.event_type,
            record.event_stream_id,
        );

        let data = record_to_event_data(&record);

        destination_connection
            .write_events(&*record.event_stream_id)
            .push_event(data)
            .execute()
            .map(|_| ())
    }).wait();

    result.map_err(|e|
        CerberusError::UserFault(
            format!("Error occured when exporting by event's type: {}", e))
    )
}

fn export_by_stream<S>(
    destination_connection: &eventstore::Connection,
    source_stream_name: &str,
    stream: S,
) -> CerberusResult<()>
    where
        S: Stream<Item=ResolvedEvent, Error=OperationError>
{
    info!("Copy stream {} ...", source_stream_name);

    let result = stream.chunks(DEFAULT_BUFFER_SIZE).for_each(|events| {
        // Events is garanteed to have at least one element.
        let first_record = events
            .as_slice()[0]
            .event
            .as_ref()
            .expect("Event field must be defined in this case 1.");

        let events = events.iter().map(|event| {
            let record = event
                .event
                .as_ref()
                .expect("Event field must be defined in this case 2.");

            record_to_event_data(record)
        });

        destination_connection
            .write_events(&*first_record.event_stream_id)
            .append_events(events)
            .execute()
            .map(|_| ())
    }).wait();

    result.map_err(|e|
        CerberusError::UserFault(
            format!("Error occured when exporting from a stream: {}", e))
    )
}

pub fn run(
    global: &clap::ArgMatches,
    params: &clap::ArgMatches,
) -> CerberusResult<()> {
    let source_connection = crate::common::create_connection_default(global)?;

    let to_tcp_port = params.value_of("to-tcp-port").unwrap_or_else(|| "1113");
    let to_host = params.value_of("to-host").expect("to-host presence is already checked by Clap");

    let to_tcp_port: u16 = to_tcp_port.parse().map_err(|e|
        CerberusError::UserFault(
            format!("--to-tcp-port parse error: {:?}", e))
    )?;

    let endpoint = format!("{}:{}", to_host, to_tcp_port).parse().map_err(|e|
        CerberusError::UserFault(
            format!("Failed to parse destination endpoint: {}", e))
    )?;

    let tpe = get_export_selection(params)?;
    let stream_name = get_stream_name(&tpe);
    let command = source_connection
        .read_stream(stream_name)
        .resolve_link_tos(eventstore::LinkTos::ResolveLink);

    let limit = get_limit(params)?;

    let stream: Box<dyn Stream<Item=ResolvedEvent, Error=OperationError>> =
        if let Limit::Top(limit) = limit {
            let stream = command
                .start_from_end_of_stream()
                .iterate_over()
                .take(limit as u64);

            Box::new(stream)
        } else {
            Box::new(command.iterate_over())
        };

    let destination_connection = eventstore::Connection::builder()
        .single_node_connection(endpoint);

    match tpe {
        Selection::StreamCategory(_) =>
            export_by_category(&source_connection, &destination_connection, stream),

        Selection::EventType(_) =>
            export_by_type(&destination_connection, stream),

        Selection::Stream(source_stream_name) =>
            export_by_stream(&destination_connection, source_stream_name, stream),
    }
}
