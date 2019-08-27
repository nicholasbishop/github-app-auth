use log::info;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::time;

const MACHINE_MAN_PREVIEW: &'static str =
    "application/vnd.github.machine-man-preview+json";

pub enum AuthError {
    JwtError(jsonwebtoken::errors::Error),
    InvalidHeaderValue(http::header::InvalidHeaderValue),
    ReqwestError(reqwest::Error),
    TimeError(time::SystemTimeError),
}

#[derive(Debug, Serialize)]
struct JwtClaims {
    /// The time that this JWT was issued
    iat: u64,
    // JWT expiration time
    exp: u64,
    // GitHub App's identifier number
    iss: u64,
}

impl JwtClaims {
    fn new(params: &GithubAuthParams) -> Result<JwtClaims, AuthError> {
        let now = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .map_err(AuthError::TimeError)?
            .as_secs();
        Ok(JwtClaims {
            // The time that this JWT was issued (now)
            iat: now,
            // JWT expiration time (1 minute from now)
            exp: now + 60,
            // GitHub App's identifier number
            iss: params.app_id,
        })
    }
}

/// This is the structure of the JSON object returned when requesting
/// an installation token.
#[derive(Debug, Deserialize)]
struct RawInstallationToken {
    token: String,
}

/// Use the app private key to generate a JWT and use the JWT to get
/// an installation token.
///
/// Reference:
/// developer.github.com/apps/building-github-apps/authenticating-with-github-apps
fn get_installation_token(
    client: &reqwest::Client,
    params: &GithubAuthParams,
) -> Result<RawInstallationToken, AuthError> {
    let claims = JwtClaims::new(params)?;
    let mut header = jsonwebtoken::Header::default();
    header.alg = jsonwebtoken::Algorithm::RS256;
    let token = jsonwebtoken::encode(&header, &claims, &params.private_key)
        .map_err(AuthError::JwtError)?;

    let url = format!(
        "https://api.github.com/app/installations/{}/access_tokens",
        params.installation_id
    );
    Ok(client
        .post(&url)
        .bearer_auth(token)
        .header("Accept", MACHINE_MAN_PREVIEW)
        .send()
        .map_err(AuthError::ReqwestError)?
        .error_for_status()
        .map_err(AuthError::ReqwestError)?
        .json()
        .map_err(AuthError::ReqwestError)?)
}

pub struct InstallationToken {
    pub client: reqwest::Client,
    token: String,
    fetch_time: time::SystemTime,
    params: GithubAuthParams,
}

impl InstallationToken {
    pub fn new(
        params: GithubAuthParams,
    ) -> Result<InstallationToken, AuthError> {
        let client = reqwest::Client::new();
        let raw = get_installation_token(&client, &params)?;
        Ok(InstallationToken {
            client,
            token: raw.token,
            fetch_time: time::SystemTime::now(),
            params: params.clone(),
        })
    }

    pub fn header(&mut self) -> Result<HeaderMap, AuthError> {
        self.refresh()?;
        let mut headers = HeaderMap::new();
        let val = format!("token {}", self.token);
        headers.insert(
            "Authorization",
            val.parse().map_err(AuthError::InvalidHeaderValue)?,
        );
        Ok(headers)
    }

    fn refresh(&mut self) -> Result<(), AuthError> {
        let elapsed = time::SystemTime::now()
            .duration_since(self.fetch_time)
            .map_err(AuthError::TimeError)?;
        // Installation tokens expire after 60 minutes. Refresh them
        // after 55 minutes to give ourselves a little wiggle room.
        if elapsed.as_secs() > (55 * 60) {
            info!("refreshing installation token");
            let raw = get_installation_token(&self.client, &self.params)?;
            self.token = raw.token;
            self.fetch_time = time::SystemTime::now();
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct GithubAuthParams {
    pub private_key: Vec<u8>,
    pub installation_id: u64,
    pub app_id: u64,
}
