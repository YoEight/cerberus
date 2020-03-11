use crate::common::{CerberusError, CerberusResult};
use eventstore::{OperationError, ResolvedEvent};
use futures::stream::Stream;
use futures::{StreamExt, TryStreamExt};

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

    Err(CerberusError::user_fault(
        "No source submitted. You should at least provide \
        --from-stream, --from-type or --from-category"
            .to_owned(),
    ))
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
        let value = value.parse().map_err(|e| {
            CerberusError::user_fault(format!("Failed to parse --top number: {}", e))
        })?;

        if value == 0 {
            return Err(CerberusError::user_fault(
                "--top parameter must be greater than 0".to_owned(),
            ));
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

async fn export_by_category<S>(
    source_connection: &eventstore::Connection,
    destination_connection: &eventstore::Connection,
    mut stream: S,
) -> CerberusResult<()>
where
    S: Stream<Item = Result<ResolvedEvent, OperationError>> + Unpin,
{
    let mut count = 1usize;
    while let Some(event) = stream.try_next().await? {
        let record = event
            .event
            .expect("Event field must be defined in this case");
        let target_stream_name = std::string::String::from_utf8_lossy(&record.data).into_owned();

        // TODO - It's possible the stream is deleted. We should skip it in
        // such a case.
        let mut inner_source = source_connection
            .read_stream(target_stream_name.as_str())
            .iterate_over_batch();

        while let Some(chunk) = inner_source.try_next().await? {
            let events = chunk.into_iter().map(|event| {
                let record = event.event.expect("Targetted event must be defined");

                record_to_event_data(&record)
            });

            info!("{} - Copy stream {} ...", count, target_stream_name);

            destination_connection
                .write_events(target_stream_name.as_str())
                .append_events(events)
                .execute()
                .await?;

            count += 1;
        }
    }

    Ok(())
}

async fn export_by_type<S>(
    destination_connection: &eventstore::Connection,
    mut stream: S,
) -> CerberusResult<()>
where
    S: Stream<Item = Result<ResolvedEvent, OperationError>> + Unpin,
{
    while let Some(event) = stream.try_next().await? {
        let record = event
            .event
            .expect("Event field must be defined in this case");

        println!(
            "Copy event {} of type {} to stream {}",
            record.event_id, record.event_type, record.event_stream_id,
        );

        let data = record_to_event_data(&record);

        destination_connection
            .write_events(&*record.event_stream_id)
            .push_event(data)
            .execute()
            .await?;
    }

    Ok(())
}

async fn export_by_stream<S>(
    destination_connection: &eventstore::Connection,
    source_stream_name: &str,
    mut stream: S,
) -> CerberusResult<()>
where
    S: Stream<Item = Result<ResolvedEvent, OperationError>> + Unpin,
{
    info!("Copy stream {} ...", source_stream_name);

    let mut buffer = Vec::with_capacity(DEFAULT_BUFFER_SIZE);
    let mut stream_name: String = "".to_string();

    // TODO - We can do much better than this. Let's use a double while loop to only have a single
    // line of code that push events to the eventstore.
    while let Some(event) = stream.try_next().await? {
        let record = event.event.expect("Event field must be defined");
        if buffer.is_empty() {
            stream_name = record.event_stream_id.clone().to_string();
        }

        buffer.push(record_to_event_data(&record));

        if buffer.len() == DEFAULT_BUFFER_SIZE {
            destination_connection
                .write_events(stream_name.as_str())
                .append_events(buffer.drain(..))
                .execute()
                .await?;
        }
    }

    if !buffer.is_empty() {
        destination_connection
            .write_events(stream_name.as_str())
            .append_events(buffer.drain(..))
            .execute()
            .await?;
    }

    Ok(())
}

pub async fn run(
    global: &clap::ArgMatches<'_>,
    params: &clap::ArgMatches<'_>,
) -> CerberusResult<()> {
    let source_connection = crate::common::create_connection_default(global).await?;

    let to_tcp_port = params.value_of("to-tcp-port").unwrap_or_else(|| "1113");
    let to_host = params
        .value_of("to-host")
        .expect("to-host presence is already checked by Clap");

    let to_tcp_port: u16 = to_tcp_port
        .parse()
        .map_err(|e| CerberusError::user_fault(format!("--to-tcp-port parse error: {:?}", e)))?;

    let endpoint = format!("{}:{}", to_host, to_tcp_port)
        .parse()
        .map_err(|e| {
            CerberusError::user_fault(format!("Failed to parse destination endpoint: {}", e))
        })?;

    let tpe = get_export_selection(params)?;
    let stream_name = get_stream_name(&tpe);
    let command = source_connection
        .read_stream(stream_name)
        .resolve_link_tos(eventstore::LinkTos::ResolveLink);

    let limit = get_limit(params)?;

    let stream: Box<dyn Stream<Item = Result<ResolvedEvent, OperationError>> + Unpin> =
        if let Limit::Top(limit) = limit {
            let stream = command
                .start_from_end_of_stream()
                .iterate_over()
                .take(limit);

            Box::new(stream)
        } else {
            Box::new(command.iterate_over())
        };

    let destination_connection = eventstore::Connection::builder()
        .single_node_connection(endpoint)
        .await;

    match tpe {
        Selection::StreamCategory(_) => {
            export_by_category(&source_connection, &destination_connection, stream).await
        }

        Selection::EventType(_) => export_by_type(&destination_connection, stream).await,

        Selection::Stream(source_stream_name) => {
            export_by_stream(&destination_connection, source_stream_name, stream).await
        }
    }
}
