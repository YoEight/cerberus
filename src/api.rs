use crate::common::{
    self,
    CerberusResult,
    CerberusError,
    SubscriptionSummary,
};

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
    pub script: String,
}

pub enum ClusterState {
    Cluster(common::ClusterMembers),
    ProblematicClusterNode,
    NoCluster,
}

fn default_error_handler<A>(mut resp: reqwest::Response) -> CerberusResult<A> {
    if resp.status().is_client_error() {
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(
                CerberusError::UserFault(
                    "Your current user cannot perform that action.".to_owned()));
        }

        let msg = resp.text().unwrap_or("<unreadable text message>".to_owned());

        return Err(
            CerberusError::UserFault(
                format!("User error: {}", msg)));
    }

    if resp.status().is_server_error() {
        let msg = resp.text().unwrap_or("<unreadable text message>".to_owned());

        return Err(
            CerberusError::UserFault(
                format!("Server error: [{}] {}", resp.status(), msg)));
    }

    Err(
        CerberusError::DevFault(
            format!("{:?}", resp)))
}

fn default_connection_error(
    api: &Api,
    error: reqwest::Error,
) -> CerberusError {
    CerberusError::UserFault(
        format!("Unable to connect to node {}:{}: {}", api.host(), api.port(), error))
}

impl<'a> Api<'a> {
    pub fn new(
        host: &'a str,
        port: u16,
        user_opt: Option<common::User>,
    ) -> Api<'a> {
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

    pub fn with_different_node<'b>(
        &self,
        host: &'b str,
        port: u16,
    ) -> Api<'b> {
        Api {
            host,
            port,
            client: self.client.clone(),
        }
    }

    pub fn create_projection(
        &self,
        conf: ProjectionConf,
    ) -> CerberusResult<common::ProjectionCreationSuccess> {

        let enabled = format!("{}", conf.enabled);
        let emit = format!("{}", conf.emit);
        let checkpoints = format!("{}", conf.checkpoints);

        let mut query = vec![
            ("enabled", enabled.as_str()),
            ("emit", emit.as_str()),
            ("checkpoints", checkpoints.as_str()),
            ("type", "JS")
        ];

        if let Some(name) = conf.name {
            query.push(("name", name));
        }

        let req = self.client
            .post(&format!("http://{}:{}/projections/{}", self.host, self.port, conf.kind))
            .header(reqwest::header::CONTENT_TYPE, "application/json;charset=UTF-8")
            .query(&query)
            .body(conf.script);

        let mut resp = req.send().map_err(|e| {
            CerberusError::UserFault(
                format!("Failed to create a projection: {}", e))
        })?;

        if resp.status().is_success() {
            return resp.json().map_err(|e| {
                let msg = resp.text().unwrap_or("<unreadable text message>".to_owned());

                CerberusError::DevFault(
                    format!(
                        "We were unable to deserialize ProjectionCreationSuccess \
                        out of a projection creation response. \
                        response body [{}], Error: {}", msg, e))
            });
        }

        default_error_handler(resp)
    }

    pub fn projection_cropped_info(
        &self,
        projection_name: &str,
    ) -> CerberusResult<common::CroppedProjectionInfo> {
        let req = self.client
            .get(&format!("http://{}:{}/projection/{}", self.host, self.port, projection_name));

        let mut resp = req.send().map_err(|e| {
            CerberusError::UserFault(
                format!("Failed to read projection info (cropped): {}", e))
        })?;

        if resp.status().is_success() {
            return resp.json().map_err(|e| {
                CerberusError::DevFault(
                    format!(
                        "We were unable to deserialize CroppedProjectionInfo out \
                        of projection info request: [{}] {:?}", e, resp))
            });
        }

        default_error_handler(resp)
    }

    pub fn node_info(&self) -> CerberusResult<common::NodeInfo> {
        let req = self.client
            .get(&format!("http://{}:{}/info?format=json", self.host, self.port));

        let mut resp = req.send().map_err(|e|
            CerberusError::UserFault(
                format!("Unable to connect to node {}:{}: {}", self.host, self.port, e))
        )?;

        if resp.status().is_success() {
            return resp.json().map_err(|e|
                CerberusError::DevFault(
                    format!("Failed to parse NodeInfo: {}", e))
            );
        }

        default_error_handler(resp)
    }

    pub fn gossip(&self) -> CerberusResult<ClusterState> {
        let req = reqwest::Client::new()
        .get(&format!("http://{}:{}/gossip?format=json", self.host, self.port));

        let mut resp = req.send().map_err(|e|
            default_connection_error(self, e)
        )?;

        if resp.status().is_success() {
            let members = resp.json().map_err(|e|
                CerberusError::DevFault(
                    format!("Failed to deserialize ClusterMembers object: {}", e))
            )?;

            return Ok(ClusterState::Cluster(members));
        }

        if resp.status().is_client_error() {
            return Ok(ClusterState::NoCluster);
        }

        if resp.status().is_server_error() {
            return Ok(ClusterState::ProblematicClusterNode);
        }

        default_error_handler(resp)
    }

    pub fn subscriptions(&self) -> CerberusResult<Vec<SubscriptionSummary>> {
        let req = self.client
            .get(&format!("http://{}:{}/subscriptions", self.host(), self.port()));

        let mut resp = req.send().map_err(|e|
            default_connection_error(self, e)
        )?;

        return resp.json().map_err(|e|
            CerberusError::DevFault(
                format!("Failed to deserialize SubscriptionSummary: {}", e))
        );
    }

    pub fn subscriptions_raw(&self) -> CerberusResult<Vec<serde_json::value::Value>> {
        let req = self.client
            .get(&format!("http://{}:{}/subscriptions", self.host(), self.port()));

        let mut resp = req.send().map_err(|e|
            default_connection_error(self, e)
        )?;

        return resp.json().map_err(|e|
            CerberusError::DevFault(
                format!("Failed to deserialize SubscriptionSummary raw: {}", e))
        );
    }

    pub fn subscription_raw(
        &self,
        stream: &str,
        group_id: &str,
    ) -> CerberusResult<serde_json::value::Value> {
        let url = format!(
            "http://{}:{}/subscriptions/{}/{}/info",
            self.host(), self.port(), stream, group_id);

        let req = self.client.get(&url);

        let mut resp = req.send().map_err(|e|
            default_connection_error(self, e)
        )?;

        return resp.json().map_err(|e|
            CerberusError::UserFault(
                format!("Failed to deserialize SubscriptionSummary: {}", e))
        );
    }
}
