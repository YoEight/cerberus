pub mod subscription {
    use crate::common::{ CerberusError, CerberusResult, User };
    use futures::Future;

    pub fn run (global: &clap::ArgMatches, params: &clap::ArgMatches, user_opt: Option<User>)
        -> CerberusResult<()>
    {

        let sub_info_opt = params.value_of("stream").and_then(|stream|
        {
            params.value_of("group-id").map(|group_id| (stream, group_id))
        });

        let (stream_name, group_id) = sub_info_opt
            .expect("Both stream and group-id params are previously checked by Clap");

        let connection = crate::common::create_connection_default(global)?;
        let mut setts = eventstore::PersistentSubscriptionSettings::default();

        setts.resolve_link_tos = params.is_present("resolve-link");
        setts.extra_stats = params.is_present("extra-stats");

        if let Some(param) = params.value_of("start-from") {
            let start_from = param.parse().map_err(|e|
            {
                CerberusError::UserFault(
                    format!("Failed to parse --start-from number parameter: {}", e))
            })?;

            setts.start_from = start_from;
        }

        if let Some(param) = params.value_of("message-timeout") {
            let millis = param.parse().map_err(|e|
            {
                CerberusError::UserFault(
                    format!("Failed to parse --message-timeout number parameter: {}", e))
            })?;

            setts.msg_timeout = std::time::Duration::from_millis(millis);
        }

        if let Some(param) = params.value_of("max-retry-count") {
            let count = param.parse().map_err(|e|
            {
                CerberusError::UserFault(
                    format!("Failed to parse --max-retry-count number parameter: {}", e))
            })?;

            setts.max_retry_count = count;
        }

        if let Some(param) = params.value_of("live-buffer-size") {
            let size = param.parse().map_err(|e|
            {
                CerberusError::UserFault(
                    format!("Failed to parse --live-buffer-size number parameter: {}", e))
            })?;

            setts.live_buf_size = size;
        }

        if let Some(param) = params.value_of("read-batch-size") {
            let size = param.parse().map_err(|e|
            {
                CerberusError::UserFault(
                    format!("Failed to parse --read-batch-size number parameter: {}", e))
            })?;

            setts.read_batch_size = size;
        }

        if let Some(param) = params.value_of("history-buffer-size") {
            let size = param.parse().map_err(|e|
            {
                CerberusError::UserFault(
                    format!("Failed to parse --history-buffer-size number parameter: {}", e))
            })?;

            setts.history_buf_size = size;
        }

        if let Some(param) = params.value_of("checkpoint-after") {
            let millis = param.parse().map_err(|e|
            {
                CerberusError::UserFault(
                    format!("Failed to parse --checkpoint-after number parameter: {}", e))
            })?;

            setts.checkpoint_after = std::time::Duration::from_millis(millis);
        }

        if let Some(param) = params.value_of("min-checkpoint-count") {
            let count = param.parse().map_err(|e|
            {
                CerberusError::UserFault(
                    format!("Failed to parse --min-checkpoint-count number parameter: {}", e))
            })?;

            setts.min_checkpoint_count = count;
        }

        if let Some(param) = params.value_of("max-checkpoint-count") {
            let count = param.parse().map_err(|e|
            {
                CerberusError::UserFault(
                    format!("Failed to parse --max-checkpoint-count number parameter: {}", e))
            })?;

            setts.max_checkpoint_count = count;
        }

        if let Some(param) = params.value_of("max-subs-count") {
            let count = param.parse().map_err(|e|
            {
                CerberusError::UserFault(
                    format!("Failed to parse --max-subs-count number parameter: {}", e))
            })?;

            setts.max_subs_count = count;
        }

        if let Some(param) = params.value_of("consumer-strategy") {
            let strategy = match param {
                "dispatch-to-single" => eventstore::SystemConsumerStrategy::DispatchToSingle,
                "round-robin" => eventstore::SystemConsumerStrategy::RoundRobin,
                "pinned" => eventstore::SystemConsumerStrategy::Pinned,
                wrong => {
                    return
                        Err(CerberusError::UserFault(
                            format!("Unknown --consumer-strategy value: [{}]", wrong)));
                },
            };

            setts.named_consumer_strategy = strategy;
        }

        let mut cmd = connection.create_persistent_subscription(stream_name, group_id)
            .settings(setts);

        if let Some(creds) = user_opt.map(|usr| usr.to_credentials()) {
            cmd = cmd.credentials(creds);
        }

        match cmd.execute().wait() {
            Err(e) => match e {
                // Very unlikely we got that error because it should be handle by
                // `eventstore::PersistActionError::AccessDenied`.
                eventstore::OperationError::AccessDenied(_) => {
                    Err(CerberusError::UserFault(
                        format!(
                            "Your current credentials doesn't allow you to create \
                            a persistent subscription on [{}] stream.", stream_name)))
                },

                eventstore::OperationError::StreamDeleted(_) => {
                    Err(CerberusError::UserFault(
                        format!(
                            "You can't create a persistent subscription on
                            [{}] stream because that strean got deleted", stream_name)))
                },

                error => {
                    Err(CerberusError::UserFault(
                        format!(
                            "Can't create a persistent subscription on [{}] \
                            stream because: {}.", stream_name, error)))
            }
            },

            Ok(result) => match result {
                eventstore::PersistActionResult::Failure(error) => match error {
                    eventstore::PersistActionError::AccessDenied =>
                        Err(CerberusError::UserFault(
                            format!(
                                "Your current credentials doesn't allow you to create \
                                a persistent subscription on [{}] stream.", stream_name))),

                    eventstore::PersistActionError::AlreadyExists =>
                        Err(CerberusError::UserFault(
                            format!(
                                "A persistent subscription already exists for the stream \
                                [{}] with the group [{}]", stream_name, group_id))),

                    // TODO - Pretty sure that use-case can't exist when creating a persistent
                    // subscription on a non existing stream. EventStore tends to not giving
                    // crap about this.
                    eventstore::PersistActionError::DoesNotExist =>
                        Err(CerberusError::UserFault(
                            format!(
                                "You can't create a persistent subscription on stream [{}] \
                                because [{}] stream doesn't exist", stream_name, stream_name))),

                    eventstore::PersistActionError::Fail =>
                        Err(CerberusError::UserFault(
                            format!(
                                "Failed to create a persistent subscription on stream \
                                [{}] with group [{}] but we don't have \
                                information on why", stream_name, group_id))),
                },

                _ => {
                    println!("Persistent subscription created.");

                    Ok(())
                }
            },
        }
    }
}

pub mod projection {
    use crate::common::{ CerberusError, CerberusResult, User };
    use reqwest::header;

    pub fn run(global: &clap::ArgMatches, params: &clap::ArgMatches, user_opt: Option<User>)
        -> CerberusResult<()>
    {
        let base_url = crate::common::create_node_uri(global);
        let kind = params.value_of("kind").expect("Kind was check by clap already");
        let enabled = format!("{}", params.is_present("enabled"));
        let emit = format!("{}", params.is_present("emit"));
        let checkpoints = format!("{}", params.is_present("checkpoints"));
        let script_filepath = params.value_of("SCRIPT").expect("SCRIPT was check by clap already");

        let script = std::fs::read_to_string(script_filepath).map_err(|e|
        {
            CerberusError::UserFault(
                format!("There was an issue with the script's filepath you submitted: {}", e))
        })?;

        let mut query = vec![
            ("enabled", enabled.as_str()),
            ("emit", emit.as_str()),
            ("checkoints", checkpoints.as_str()),
            ("type", "JS")
        ];

        if let Some(name) = params.value_of("name") {
            query.push(("name", name));
        }

        let mut req = reqwest::Client::new()
            .post(&format!("{}/projections/{}", base_url, kind))
            .header(header::CONTENT_TYPE, "application/json;charset=UTF-8")
            .query(&query)
            .body(script);

        if let Some(user) = user_opt {
            req = req.basic_auth(user.login, user.password);
        }

        let mut resp = req.send().map_err(|e| {
            CerberusError::UserFault(
                format!("Failed to create a projection: {}", e))
        })?;

        if resp.status().as_u16() == 401 {
            return Err(
                CerberusError::UserFault(
                    "Your current user cannot create a projection.".to_owned()));
        }

        // let result: serde_json::value::Value = resp.json().unwrap();
        let result = resp.text().unwrap();
        // serde_json::to_writer_pretty(std::io::stdout(), &result).unwrap();
        println!("Result: {}", result);

        // println!("Projection is created: {:?}", resp);

        Ok(())
    }
}
