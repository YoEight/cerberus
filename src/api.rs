use crate::common::{
    self,
    CerberusResult,
    CerberusError,
};

pub struct Api<'a> {
    base_url: &'a str,
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

impl<'a> Api<'a> {
    pub fn new(
        base_url: &'a str,
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
            base_url,
            client: builder.build().unwrap(),
        }
    }

    pub fn with_different_base_url<'b>(
        &self,
        base_url: &'b str,
    ) -> Api<'b> {
        Api {
            base_url,
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
            .post(&format!("{}/projections/{}", self.base_url, conf.kind))
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
            .get(&format!("{}/projection/{}", self.base_url, projection_name));

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
}
