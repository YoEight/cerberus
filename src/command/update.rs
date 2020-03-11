pub mod subscription {
    use crate::common::{CerberusError, CerberusResult, User};

    pub async fn run(
        global: &clap::ArgMatches<'_>,
        params: &clap::ArgMatches<'_>,
        user_opt: Option<User<'_>>,
    ) -> CerberusResult<()> {
        let sub_info_opt = params.value_of("stream").and_then(|stream| {
            params
                .value_of("group-id")
                .map(|group_id| (stream, group_id))
        });

        let (stream_name, group_id) =
            sub_info_opt.expect("Both stream and group-id params are previously checked by Clap");

        let connection = crate::common::create_connection_default(global).await?;
        let mut setts = eventstore::PersistentSubscriptionSettings::default();

        setts.resolve_link_tos = params.is_present("resolve-link");
        setts.extra_stats = params.is_present("extra-stats");

        if let Some(param) = params.value_of("start-from") {
            let start_from = param.parse().map_err(|e| {
                CerberusError::user_fault(format!(
                    "Failed to parse --start-from number parameter: {}",
                    e
                ))
            })?;

            setts.start_from = start_from;
        }

        if let Some(param) = params.value_of("message-timeout") {
            let millis = param.parse().map_err(|e| {
                CerberusError::user_fault(format!(
                    "Failed to parse --message-timeout number parameter: {}",
                    e
                ))
            })?;

            setts.msg_timeout = std::time::Duration::from_millis(millis);
        }

        if let Some(param) = params.value_of("max-retry-count") {
            let count = param.parse().map_err(|e| {
                CerberusError::user_fault(format!(
                    "Failed to parse --max-retry-count number parameter: {}",
                    e
                ))
            })?;

            setts.max_retry_count = count;
        }

        if let Some(param) = params.value_of("live-buffer-size") {
            let size = param.parse().map_err(|e| {
                CerberusError::user_fault(format!(
                    "Failed to parse --live-buffer-size number parameter: {}",
                    e
                ))
            })?;

            setts.live_buf_size = size;
        }

        if let Some(param) = params.value_of("read-batch-size") {
            let size = param.parse().map_err(|e| {
                CerberusError::user_fault(format!(
                    "Failed to parse --read-batch-size number parameter: {}",
                    e
                ))
            })?;

            setts.read_batch_size = size;
        }

        if let Some(param) = params.value_of("history-buffer-size") {
            let size = param.parse().map_err(|e| {
                CerberusError::user_fault(format!(
                    "Failed to parse --history-buffer-size number parameter: {}",
                    e
                ))
            })?;

            setts.history_buf_size = size;
        }

        if let Some(param) = params.value_of("checkpoint-after") {
            let millis = param.parse().map_err(|e| {
                CerberusError::user_fault(format!(
                    "Failed to parse --checkpoint-after number parameter: {}",
                    e
                ))
            })?;

            setts.checkpoint_after = std::time::Duration::from_millis(millis);
        }

        if let Some(param) = params.value_of("min-checkpoint-count") {
            let count = param.parse().map_err(|e| {
                CerberusError::user_fault(format!(
                    "Failed to parse --min-checkpoint-count number parameter: {}",
                    e
                ))
            })?;

            setts.min_checkpoint_count = count;
        }

        if let Some(param) = params.value_of("max-checkpoint-count") {
            let count = param.parse().map_err(|e| {
                CerberusError::user_fault(format!(
                    "Failed to parse --max-checkpoint-count number parameter: {}",
                    e
                ))
            })?;

            setts.max_checkpoint_count = count;
        }

        if let Some(param) = params.value_of("max-subs-count") {
            let count = param.parse().map_err(|e| {
                CerberusError::user_fault(format!(
                    "Failed to parse --max-subs-count number parameter: {}",
                    e
                ))
            })?;

            setts.max_subs_count = count;
        }

        if let Some(param) = params.value_of("consumer-strategy") {
            let strategy = match param {
                "dispatch-to-single" => eventstore::SystemConsumerStrategy::DispatchToSingle,
                "round-robin" => eventstore::SystemConsumerStrategy::RoundRobin,
                "pinned" => eventstore::SystemConsumerStrategy::Pinned,
                wrong => {
                    return Err(CerberusError::user_fault(format!(
                        "Unknown --consumer-strategy value: [{}]",
                        wrong
                    )));
                }
            };

            setts.named_consumer_strategy = strategy;
        }

        let mut cmd = connection
            .update_persistent_subscription(stream_name, group_id)
            .settings(setts);

        if let Some(creds) = user_opt.map(|usr| usr.to_credentials()) {
            cmd = cmd.credentials(creds);
        }

        match cmd.execute().await {
            Err(e) => match e {
                // Very unlikely we got that error because it should be handle by
                // `eventstore::PersistActionError::AccessDenied`.
                eventstore::OperationError::AccessDenied(_) => {
                    Err(CerberusError::user_fault(format!(
                        "Your current credentials doesn't allow you to update \
                            a persistent subscription on [{}] stream.",
                        stream_name
                    )))
                }

                eventstore::OperationError::StreamDeleted(_) => {
                    Err(CerberusError::user_fault(format!(
                        "You can't create a persistent subscription on
                            [{}] stream because that strean got deleted",
                        stream_name
                    )))
                }

                error => Err(CerberusError::user_fault(format!(
                    "Can't create a persistent subscription on [{}] \
                            stream because: {}.",
                    stream_name, error
                ))),
            },

            Ok(result) => match result {
                eventstore::PersistActionResult::Failure(error) => match error {
                    eventstore::PersistActionError::AccessDenied => {
                        Err(CerberusError::user_fault(format!(
                            "Your current credentials doesn't allow you to update \
                                a persistent subscription on [{}] stream.",
                            stream_name
                        )))
                    }

                    eventstore::PersistActionError::AlreadyExists => {
                        Err(CerberusError::user_fault(format!(
                            "A persistent subscription already exists for the stream \
                                [{}] with the group [{}]",
                            stream_name, group_id
                        )))
                    }

                    eventstore::PersistActionError::DoesNotExist => {
                        Err(CerberusError::user_fault(format!(
                            "You can't update a persistent subscription on stream [{}] \
                                with group-id [{}] because the subscription doesn't exist",
                            stream_name, group_id
                        )))
                    }

                    eventstore::PersistActionError::Fail => Err(CerberusError::user_fault(format!(
                        "Failed to update a persistent subscription on stream \
                                [{}] with group [{}] but we don't have \
                                information on why",
                        stream_name, group_id
                    ))),
                },

                _ => {
                    println!("Persistent subscription updated.");

                    Ok(())
                }
            },
        }
    }
}
