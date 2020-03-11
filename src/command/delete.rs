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
        let mut cmd = connection.delete_persistent_subscription(stream_name, group_id);

        if let Some(creds) = user_opt.map(|usr| usr.to_credentials()) {
            cmd = cmd.credentials(creds);
        }

        match cmd.execute().await {
            Err(e) => match e {
                // Very unlikely we got that error because it should be handle by
                // `eventstore::PersistActionError::AccessDenied`.
                eventstore::OperationError::AccessDenied(_) => {
                    Err(CerberusError::user_fault(format!(
                        "Your current credentials doesn't allow you to delete \
                            a persistent subscription on [{}] stream.",
                        stream_name
                    )))
                }

                error => Err(CerberusError::user_fault(format!(
                    "Can't delete a persistent subscription on [{}] \
                            stream because: {}.",
                    stream_name, error
                ))),
            },

            Ok(result) => match result {
                eventstore::PersistActionResult::Failure(error) => match error {
                    eventstore::PersistActionError::AccessDenied => {
                        Err(CerberusError::user_fault(format!(
                            "Your current credentials doesn't allow you to delete \
                                a persistent subscription on [{}] stream.",
                            stream_name
                        )))
                    }

                    // Unlikely to happen, considering we try to delete a persistent
                    // subscription.
                    eventstore::PersistActionError::AlreadyExists => unreachable!(),

                    eventstore::PersistActionError::DoesNotExist => {
                        Err(CerberusError::user_fault(format!(
                            "You can't delete a persistent subscription on stream [{}] \
                                with group id [{}] because it doesn't exist",
                            stream_name, group_id
                        )))
                    }

                    eventstore::PersistActionError::Fail => Err(CerberusError::user_fault(format!(
                        "Failed to delete a persistent subscription on stream \
                                [{}] with group [{}] but we don't have \
                                information on why",
                        stream_name, group_id
                    ))),
                },

                _ => {
                    println!("Persistent subscription deleted.");

                    Ok(())
                }
            },
        }
    }
}
