use crate::api::{self, Api};
use crate::common::{CerberusError, CerberusResult};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;

#[derive(Serialize, Deserialize, Debug)]
struct PersistentSubscriptionSettings {
    #[serde(default)]
    pub resolve_link: bool,

    pub start_from: i64,

    #[serde(default)]
    pub extra_stats: bool,
    pub msg_timeout_in_ms: Option<usize>,
    pub max_retry_count: Option<usize>,
    pub live_buffer_size: Option<usize>,
    pub read_batch_size: Option<usize>,
    pub buffer_size: Option<usize>,
    pub checkpoint_after_in_ms: Option<usize>,
    pub min_checkpoint_count: Option<usize>,
    pub max_checkpoint_count: Option<usize>,
    pub max_subs_count: Option<usize>,

    #[serde(default)]
    pub strategy: SubscriptionStrategy,
}

impl PersistentSubscriptionSettings {
    fn to_sub_config(&self) -> api::SubscriptionConfig {
        let named_consumer_strategy = match self.strategy {
            SubscriptionStrategy::RoundRobin => api::NamedConsumerStrategy::RoundRobin,
            SubscriptionStrategy::DispatchToSingle => api::NamedConsumerStrategy::DispatchToSingle,
            SubscriptionStrategy::Pinned => api::NamedConsumerStrategy::Pinned,
        };

        api::SubscriptionConfig {
            resolve_linktos: self.resolve_link,
            start_from: self.start_from,
            message_timeout_milliseconds: self.msg_timeout_in_ms.unwrap_or(10_000),
            extra_statistics: self.extra_stats,
            max_retry_count: self.max_retry_count.unwrap_or(10),
            live_buffer_size: self.live_buffer_size.unwrap_or(500),
            buffer_size: self.buffer_size.unwrap_or(500),
            read_batch_size: self.read_batch_size.unwrap_or(20),
            check_point_after_milliseconds: self.checkpoint_after_in_ms.unwrap_or(1_000),
            min_check_point_count: self.min_checkpoint_count.unwrap_or(10),
            max_check_point_count: self.max_checkpoint_count.unwrap_or(500),
            max_subscriber_count: self.max_subs_count.unwrap_or(10),
            named_consumer_strategy,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
enum SubscriptionStrategy {
    RoundRobin,
    DispatchToSingle,
    Pinned,
}

impl Default for SubscriptionStrategy {
    fn default() -> Self {
        SubscriptionStrategy::RoundRobin
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum ProjectionType {
    Continuous,
    OneTime,
}

impl ProjectionType {
    fn get_human_string(self) -> &'static str {
        match self {
            ProjectionType::Continuous => "continuous",
            ProjectionType::OneTime => "onetime",
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Subscription {
    stream: String,
    group: String,

    #[serde(flatten)]
    settings: PersistentSubscriptionSettings,
}

#[derive(Serialize, Deserialize, Debug)]
struct Projection {
    name: String,
    path: String,

    #[serde(rename = "type")]
    tpe: ProjectionType,

    #[serde(default)]
    emit: bool,

    #[serde(default)]
    enabled: bool,

    #[serde(default)]
    checkpoints: bool,

    #[serde(default)]
    track_emitted_streams: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct Compliance {
    #[serde(rename = "projection")]
    #[serde(default)]
    projections: Vec<Projection>,

    #[serde(rename = "subscription")]
    #[serde(default)]
    subscriptions: Vec<Subscription>,
}

pub async fn run(
    _: &clap::ArgMatches<'_>,
    params: &clap::ArgMatches<'_>,
    api: Api<'_>,
) -> CerberusResult<()> {
    let filepath = params.value_of("file").expect("Already checked by clap");

    let mut file = File::open(filepath)?;
    let mut buffer: Vec<u8> = Vec::new();
    let dry_run = params.is_present("dry-run");

    file.read_to_end(&mut buffer)?;

    let compliance: Compliance = toml::from_slice(buffer.as_slice())?;

    if !compliance.subscriptions.is_empty() {
        println!("Subscriptions");
        println!("=============");

        for sub in compliance.subscriptions.iter() {
            let mut conf = sub.settings.to_sub_config();
            let detail_opt = api
                .subscription_opt(sub.stream.as_str(), sub.group.as_str())
                .await?;

            match detail_opt {
                None => {
                    println!(
                        "\t{} Subscription on [{}] with group [{}] doesn't exist.",
                        "⨯⨯⨯".red(),
                        sub.stream,
                        sub.group
                    );

                    if dry_run {
                        println!(
                            "\t\t{}",
                            "[DRY-RUN] We would have created that subscription".yellow()
                        );
                    } else {
                        println!("\t\tCreating…");

                        api.create_subscription(sub.stream.as_str(), sub.group.as_str(), conf)
                            .await?;

                        println!("\t\t{}", "Subscription is created".green());
                    }
                }

                Some(detail) => {
                    // We decide to ignore the very high probability that
                    // start_from on the server-side configuration is greater.
                    // TODO - Propose to override that behaviour through a setting.
                    conf.start_from = detail.config.start_from;

                    if conf != detail.config {
                        println!(
                            "\t{} Subscription on [{}] with group [{}] exists but has a different configuration.",
                            "‐‐‐".yellow(),
                            sub.stream,
                            sub.group);

                        let lhs = serde_json::to_string_pretty(&conf).unwrap();
                        let rhs = serde_json::to_string_pretty(&detail.config).unwrap();

                        println!("\t\tConfiguration differences:");
                        println!("\t\t-------------------------");

                        for d in diff::lines(lhs.as_str(), rhs.as_str()) {
                            match d {
                                diff::Result::Left(l) => {
                                    println!("\t\t\t{} {}", "+".green(), l.green())
                                }

                                diff::Result::Right(r) => {
                                    println!("\t\t\t{} {}", "-".red(), r.red())
                                }

                                diff::Result::Both(same, _) => println!("\t\t\t {}", same),
                            }
                        }

                        if dry_run {
                            println!(
                                "\n\t\t{}",
                                "[DRY-RUN] We would have updated that subscription{}".yellow()
                            );
                        } else {
                            println!("\n\t\t Updating…");

                            api.update_subscription(sub.stream.as_str(), sub.group.as_str(), conf)
                                .await?;

                            println!("\n\t\t{}", "Subscription is updated.".green());
                        }
                    } else {
                        println!(
                            "\t{} Subscription on [{}] with group [{}] is up-to-date.",
                            "✓✓✓".green(),
                            sub.stream,
                            sub.group
                        );
                    }
                }
            }
        }
    }

    if !compliance.projections.is_empty() {
        println!("Projections");
        println!("===========");

        for proj in compliance.projections {
            if let Some(server_proj_info) =
                api.projection_cropped_info_opt(proj.name.as_str()).await?
            {
                if server_proj_info.status == "Faulted" {
                    let reason = server_proj_info
                        .reason
                        .unwrap_or_else(|| "<No reason given>".to_owned());

                    println!(
                        "\t{} Projection [{}] exists but is in faulted status:",
                        "‐‐‐".yellow(),
                        proj.name
                    );

                    println!("\t{}", reason.red());

                    // Check for configuration differences and see if we can
                    // improve the situation.
                    perform_projection_checks(&api, &proj, dry_run).await?;
                } else {
                    // Supposedly a live status.
                    perform_projection_checks(&api, &proj, dry_run).await?;
                }
            } else {
                println!(
                    "\t{} Projection [{}] doesn't exist.",
                    "⨯⨯⨯".red(),
                    proj.name
                );

                if dry_run {
                    println!(
                        "\t\t{}",
                        "[DRY-RUN] We would have created that projection".yellow()
                    );
                } else {
                    println!("\t\tCreating…");

                    let mut buffer: Vec<u8> = Vec::new();
                    let mut file = File::open(proj.path.as_str())?;

                    file.read_to_end(&mut buffer)?;

                    let proj_code = std::str::from_utf8(&buffer).map_err(|e| {
                        CerberusError::UserFault(format!(
                            "Projection code located at {} is not encoded in UTF-8: {}",
                            proj.path, e
                        ))
                    })?;

                    let conf = api::ProjectionConf {
                        name: Some(proj.name.as_str()),
                        kind: proj.tpe.get_human_string(),
                        enabled: proj.enabled,
                        emit: proj.emit,
                        checkpoints: proj.checkpoints,
                        track_emitted_streams: proj.track_emitted_streams,
                        script: proj_code.to_owned(),
                    };

                    api.create_projection(conf).await?;

                    println!(
                        "{}",
                        format!("\t\tProjection on [{}] is created.", proj.name).green()
                    );
                }
            }
        }
    }

    Ok(())
}

async fn perform_projection_checks(
    api: &Api<'_>,
    proj: &Projection,
    dry_run: bool,
) -> CerberusResult<()> {
    let proj_config = api.projection_config(proj.name.as_str()).await?;
    let mut buffer: Vec<u8> = Vec::new();
    let mut file = File::open(proj.path.as_str())?;

    file.read_to_end(&mut buffer)?;

    let latest_proj_code = std::str::from_utf8(&buffer).map_err(|e| {
        CerberusError::UserFault(format!(
            "Projection code located at {} is not encoded in UTF-8: {}",
            proj.path, e
        ))
    })?;

    let diffs = diff::lines(latest_proj_code, proj_config.query.as_str());
    let has_code_differences = diffs.iter().any(|result| match result {
        diff::Result::Left(_) => true,
        diff::Result::Right(_) => true,
        _ => false,
    });

    if has_code_differences {
        println!(
            "\t{} Projection [{}] exists but has code differences:",
            "‐‐‐".yellow(),
            proj.name
        );

        println!("\t\tCode differences:");
        println!("\t\t----------------");

        for d in diffs {
            match d {
                diff::Result::Left(l) => println!("\t\t\t{} {}", "+".green(), l.green()),

                diff::Result::Right(r) => println!("\t\t\t{} {}", "-".red(), r.red()),

                diff::Result::Both(same, _) => println!("\t\t\t {}", same),
            }
        }

        if dry_run {
            println!(
                "\n\t\t{}",
                "[DRY-RUN] We would have updated that projection".yellow()
            );
        } else {
            println!("\n\t\tUpdating…");

            let conf = api::UpdateProjectionConf {
                name: proj.name.as_str(),
                emit: proj.emit,
                track_emitted_streams: proj.track_emitted_streams,
                query: latest_proj_code,
            };

            api.update_projection_query(conf).await?;

            println!("\n\t\t{}", "Projection query is updated.".green());
        }
    } else {
        println!(
            "\t{} Projection [{}] is up-to-date.",
            "✓✓✓".green(),
            proj.name
        );
    }

    Ok(())
}
