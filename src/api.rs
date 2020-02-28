use crate::common::{
    self, CerberusError, CerberusResult, Projection, Projections, SubscriptionSummary,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDetail {
    pub event_stream_id: String,
    pub group_name: String,
    pub config: SubscriptionConfig,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum NamedConsumerStrategy {
    RoundRobin,
    DispatchToSingle,
    Pinned,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionConfig {
    pub resolve_linktos: bool,
    pub start_from: i64,
    pub message_timeout_milliseconds: usize,
    pub extra_statistics: bool,
    pub max_retry_count: usize,
    pub live_buffer_size: usize,
    pub buffer_size: usize,
    pub read_batch_size: usize,
    pub check_point_after_milliseconds: usize,
    pub min_check_point_count: usize,
    pub max_check_point_count: usize,
    pub max_subscriber_count: usize,
    pub named_consumer_strategy: NamedConsumerStrategy,
}

impl Default for SubscriptionConfig {
    fn default() -> Self {
        SubscriptionConfig {
            resolve_linktos: false,
            start_from: 0,
            message_timeout_milliseconds: 10_000,
            extra_statistics: false,
            max_retry_count: 10,
            live_buffer_size: 500,
            buffer_size: 500,
            read_batch_size: 20,
            check_point_after_milliseconds: 1_000,
            min_check_point_count: 10,
            max_check_point_count: 500,
            max_subscriber_count: 10,
            named_consumer_strategy: NamedConsumerStrategy::RoundRobin,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionConfig {
    pub name: String,
    pub query: String,

    #[serde(default)]
    pub emit_enabled: bool,
}

pub struct Api<'a> {
    host: &'a str,
    port: u16,
    client: reqwest::Client,
}

pub struct ProjectionConf<'a> {
    pub name: Option<&'a str>,
    pub kind: &'a str,
    pub enabled: bool,
    pub emit: bool,
    pub checkpoints: bool,
    pub track_emitted_streams: bool,
    pub script: String,
}

pub struct UpdateProjectionConf<'a> {
    pub name: &'a str,
    pub emit: bool,
    pub track_emitted_streams: bool,
    pub query: &'a str,
}

pub enum ClusterState {
    Cluster(common::ClusterMembers),
    ProblematicClusterNode,
    NoCluster,
}

async fn default_error_handler<A>(resp: reqwest::Response) -> CerberusResult<A> {
    if resp.status().is_client_error() {
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(CerberusError::user_fault(
                "Your current user cannot perform that action.",
            ));
        }

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(CerberusError::user_fault(
                "You are asking for a resource that doesn't exist.",
            ));
        }

        let msg = resp
            .text()
            .await
            .unwrap_or_else(|_| "<unreadable text message>".to_owned());

        return Err(CerberusError::user_fault(format!("User error: {}", msg)));
    }

    let status = resp.status();

    if status.is_server_error() {
        let msg = resp
            .text()
            .await
            .unwrap_or_else(|_| "<unreadable text message>".to_owned());

        return Err(CerberusError::user_fault(format!(
            "Server error: [{}] {}",
            status, msg
        )));
    }

    Err(CerberusError::dev_fault(format!("{:?}", resp)))
}

fn default_connection_error(api: &Api, error: reqwest::Error) -> CerberusError {
    CerberusError::UserFault(format!(
        "Unable to connect to node {}:{}: {}",
        api.host(),
        api.port(),
        error
    ))
}

impl<'a> Api<'a> {
    pub fn new(host: &'a str, port: u16, user_opt: Option<common::User>) -> Api<'a> {
        let mut builder = reqwest::Client::builder();

        if let Some(user) = user_opt {
            let mut headers = reqwest::header::HeaderMap::new();
            let auth = match user.password {
                None => format!("{}:", user.login),
                Some(passw) => format!("{}:{}", user.login, passw),
            };

            let header_value = format!("Basic {}", base64::encode(&auth));
            let header_value = reqwest::header::HeaderValue::from_str(&header_value).unwrap();

            headers.insert(reqwest::header::AUTHORIZATION, header_value);
            builder = builder.default_headers(headers);
        }

        Api {
            host,
            port,
            client: builder.build().unwrap(),
        }
    }

    pub fn host(&self) -> &str {
        self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn with_different_node<'b>(&self, host: &'b str, port: u16) -> Api<'b> {
        Api {
            host,
            port,
            client: self.client.clone(),
        }
    }

    pub async fn create_projection(
        &'a self,
        conf: ProjectionConf<'a>,
    ) -> CerberusResult<common::ProjectionCreationSuccess> {
        let enabled = format!("{}", conf.enabled);
        let emit = format!("{}", conf.emit);
        let checkpoints = format!("{}", conf.checkpoints);
        let track_emitted_streams_value = format!("{}", conf.track_emitted_streams);

        let mut query = vec![
            ("enabled", enabled.as_str()),
            ("emit", emit.as_str()),
            ("checkpoints", checkpoints.as_str()),
            ("trackemittedstreams", track_emitted_streams_value.as_str()),
            ("type", "JS"),
        ];

        if let Some(name) = conf.name {
            query.push(("name", name));
        }

        let req = self
            .client
            .post(&format!(
                "http://{}:{}/projections/{}",
                self.host, self.port, conf.kind
            ))
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/json;charset=UTF-8",
            )
            .query(&query)
            .body(conf.script);

        let resp = req.send().await.map_err(|e| {
            CerberusError::user_fault(format!("Failed to create a projection: {}", e))
        })?;

        if resp.status().is_success() {
            return resp.json().await.map_err(|e| {
                CerberusError::dev_fault(format!(
                    "We were unable to deserialize ProjectionCreationSuccess \
                    out of a projection creation response. Error: {}",
                    e
                ))
            });
        }

        default_error_handler(resp).await
    }

    pub async fn projection_cropped_info(
        &self,
        projection_name: &str,
    ) -> CerberusResult<common::CroppedProjectionInfo> {
        let info_opt = self.projection_cropped_info_opt(projection_name).await?;

        match info_opt {
            Some(info) => Ok(info),

            None => Err(Box::new(CerberusError::UserFault(format!(
                "Projection [{}] doesn't exist.",
                projection_name
            )))),
        }
    }

    pub async fn projection_cropped_info_opt(
        &self,
        projection_name: &str,
    ) -> CerberusResult<Option<common::CroppedProjectionInfo>> {
        let req = self.client.get(&format!(
            "http://{}:{}/projection/{}",
            self.host, self.port, projection_name
        ));

        let resp = req.send().await.map_err(|e| {
            CerberusError::UserFault(format!("Failed to read projection info (cropped): {}", e))
        })?;

        if resp.status().is_success() {
            let info = resp.json().await.map_err(|e| {
                CerberusError::dev_fault(format!(
                    "We were unable to deserialize CroppedProjectionInfo out \
                        of projection info request: [{}]",
                    e
                ))
            })?;

            return Ok(Some(info));
        }

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        default_error_handler(resp).await
    }

    pub async fn node_info(&self) -> CerberusResult<common::NodeInfo> {
        let req = self.client.get(&format!(
            "http://{}:{}/info?format=json",
            self.host, self.port
        ));

        let resp = req.send().await?;

        if resp.status().is_success() {
            return resp
                .json()
                .await
                .map_err(|e| CerberusError::dev_fault(format!("Failed to parse NodeInfo: {}", e)));
        }

        default_error_handler(resp).await
    }

    pub async fn gossip(&self) -> CerberusResult<ClusterState> {
        let req = reqwest::Client::new().get(&format!(
            "http://{}:{}/gossip?format=json",
            self.host, self.port
        ));

        let resp = req
            .send()
            .await
            .map_err(|e| default_connection_error(self, e))?;

        if resp.status().is_success() {
            let members = resp.json().await.map_err(|e| {
                CerberusError::dev_fault(format!(
                    "Failed to deserialize ClusterMembers object: {}",
                    e
                ))
            })?;

            return Ok(ClusterState::Cluster(members));
        }

        if resp.status().is_client_error() {
            return Ok(ClusterState::NoCluster);
        }

        if resp.status().is_server_error() {
            return Ok(ClusterState::ProblematicClusterNode);
        }

        default_error_handler(resp).await
    }

    pub async fn subscriptions(&self) -> CerberusResult<Vec<SubscriptionSummary>> {
        let req = self.client.get(&format!(
            "http://{}:{}/subscriptions",
            self.host(),
            self.port()
        ));

        let resp = req
            .send()
            .await
            .map_err(|e| default_connection_error(self, e))?;

        if resp.status().is_success() {
            return resp.json().await.map_err(|e| {
                CerberusError::dev_fault(format!("Failed to deserialize SubscriptionSummary: {}", e))
            });
        }

        default_error_handler(resp).await
    }

    pub async fn subscriptions_raw(&self) -> CerberusResult<Vec<serde_json::value::Value>> {
        let req = self.client.get(&format!(
            "http://{}:{}/subscriptions",
            self.host(),
            self.port()
        ));

        let resp = req
            .send()
            .await
            .map_err(|e| default_connection_error(self, e))?;

        if resp.status().is_success() {
            return resp.json().await.map_err(|e| {
                CerberusError::dev_fault(format!(
                    "Failed to deserialize SubscriptionSummary raw: {}",
                    e
                ))
            });
        }

        default_error_handler(resp).await
    }

    pub async fn subscription_raw(
        &self,
        stream: &str,
        group_id: &str,
    ) -> CerberusResult<serde_json::value::Value> {
        let url = format!(
            "http://{}:{}/subscriptions/{}/{}/info",
            self.host(),
            self.port(),
            stream,
            group_id
        );

        let req = self.client.get(&url);

        let resp = req
            .send()
            .await
            .map_err(|e| default_connection_error(self, e))?;

        if resp.status().is_success() {
            return resp.json().await.map_err(|e| {
                CerberusError::dev_fault(format!("Failed to deserialize SubscriptionSummary: {}", e))
            });
        }

        default_error_handler(resp).await
    }

    // Note used at the moment but will be later.
    async fn _subscription(
        &self,
        stream: &str,
        group_id: &str,
    ) -> CerberusResult<SubscriptionDetail> {
        let sub_opt = self.subscription_opt(stream, group_id).await?;

        match sub_opt {
            Some(sub) => Ok(sub),

            None => Err(CerberusError::user_fault(format!(
                "Persistent subscription targetting [{}] stream on group [{}] doesn't exist.",
                stream, group_id
            ))),
        }
    }

    pub async fn subscription_opt(
        &self,
        stream: &str,
        group_id: &str,
    ) -> CerberusResult<Option<SubscriptionDetail>> {
        let url = format!(
            "http://{}:{}/subscriptions/{}/{}/info",
            self.host(),
            self.port(),
            stream,
            group_id
        );

        let req = self.client.get(&url);

        let resp = req
            .send()
            .await
            .map_err(|e| default_connection_error(self, e))?;

        if resp.status().is_success() {
            let detail = resp.json().await.map_err(|e| {
                CerberusError::dev_fault(format!("Failed to deserialize SubscriptionDetail: {}", e))
            })?;

            return Ok(Some(detail));
        }

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        default_error_handler(resp).await
    }

    pub async fn projections(&self, kind: &str) -> CerberusResult<Vec<Projection>> {
        let req = self.client.get(&format!(
            "http://{}:{}/projections/{}",
            self.host, self.port, kind
        ));

        let resp = req
            .send()
            .await
            .map_err(|e| default_connection_error(self, e))?;

        if resp.status().is_success() {
            let result: Projections = resp.json().await.map_err(|e| {
                CerberusError::dev_fault(format!("Failed to deserialize Projections: {}", e))
            })?;

            return Ok(result.projections);
        }

        default_error_handler(resp).await
    }

    pub async fn create_subscription(
        &self,
        stream: &str,
        group: &str,
        config: SubscriptionConfig,
    ) -> CerberusResult<()> {
        let url = format!(
            "http://{}:{}/subscriptions/{}/{}",
            self.host, self.port, stream, group
        );

        let req = self
            .client
            .put(&url)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/json;charset=UTF-8",
            )
            .body(serde_json::to_vec(&config).unwrap());

        let resp = req
            .send()
            .await
            .map_err(|e| default_connection_error(self, e))?;

        if resp.status().is_success() {
            return Ok(());
        }

        default_error_handler(resp).await
    }

    pub async fn update_subscription(
        &self,
        stream: &str,
        group: &str,
        config: SubscriptionConfig,
    ) -> CerberusResult<()> {
        let url = format!(
            "http://{}:{}/subscriptions/{}/{}",
            self.host, self.port, stream, group
        );

        let req = self
            .client
            .post(&url)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/json;charset=UTF-8",
            )
            .body(serde_json::to_vec(&config).unwrap());

        let resp = req
            .send()
            .await
            .map_err(|e| default_connection_error(self, e))?;

        if resp.status().is_success() {
            return Ok(());
        }

        default_error_handler(resp).await
    }

    pub async fn projection_config(
        &self,
        projection_name: &str,
    ) -> CerberusResult<ProjectionConfig> {
        let url = format!(
            "http://{}:{}/projection/{}/query",
            self.host(),
            self.port(),
            projection_name
        );

        let query = vec![("config", "yes")];

        let req = self.client.get(&url).query(&query);

        let resp = req
            .send()
            .await
            .map_err(|e| default_connection_error(self, e))?;

        if resp.status().is_success() {
            return resp.json().await.map_err(|e| {
                CerberusError::dev_fault(format!("Failed to deserialize ProjectionConfig: {}", e))
            });
        }

        default_error_handler(resp).await
    }

    pub async fn update_projection_query(
        &'a self,
        conf: UpdateProjectionConf<'a>,
    ) -> CerberusResult<()> {
        let url = format!(
            "http://{}:{}/projection/{}/query",
            self.host(),
            self.port(),
            conf.name
        );

        let emit_value = format!("{}", conf.emit);
        let track_emitted_streams_value = format!("{}", conf.track_emitted_streams);
        let query_params = vec![
            ("emit", emit_value.as_str()),
            ("trackemittedstreams", track_emitted_streams_value.as_str()),
            ("type", "js"),
        ];

        let req = self
            .client
            .put(url.as_str())
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/json;charset=UTF-8",
            )
            .query(&query_params)
            .body(conf.query.to_owned());

        let resp = req
            .send()
            .await
            .map_err(|e| default_connection_error(self, e))?;

        if resp.status().is_success() {
            return Ok(());
        }

        default_error_handler(resp).await
    }
}
