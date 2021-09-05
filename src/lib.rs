//! This crate provides a library for authenticating with the GitHub
//! API as a GitHub app. See
//! [Authenticating with GitHub Apps](https://developer.github.com/apps/building-github-apps/authenticating-with-github-apps)
//! for details about the authentication flow.
//!
//! Example:
//!
//! ```no_run
//! use github_app_auth::{GithubAuthParams, InstallationAccessToken};
//!
//! // The token is mutable because the installation access token must be
//! // periodically refreshed. See the `GithubAuthParams` documentation
//! // for details on how to get the private key and the two IDs.
//! let mut token = InstallationAccessToken::new(GithubAuthParams {
//!     user_agent: "my-cool-user-agent".into(),
//!     private_key: b"my private key".to_vec(),
//!     app_id: 1234,
//!     installation_id: 5678,
//! }).expect("failed to get installation access token");
//!
//! // Getting the authentication header will automatically refresh
//! // the token if necessary, but of course this operation can fail.
//! let header = token.header().expect("failed to get authentication header");
//!
//! token.client.post("https://some-github-api-url").headers(header).send();
//! ```
#![warn(missing_docs)]

use chrono::{DateTime, Duration, Utc};
use log::info;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::time;

const MACHINE_MAN_PREVIEW: &str =
    "application/vnd.github.machine-man-preview+json";

/// Authentication error enum.
#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    /// An error occurred when trying to encode the JWT.
    #[error("JWT encoding failed")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    /// The token cannot be encoded as an HTTP header.
    #[error("HTTP header encoding failed")]
    InvalidHeaderValue(#[from] http::header::InvalidHeaderValue),

    /// An HTTP request failed.
    #[error("HTTP request failed")]
    ReqwestError(#[from] reqwest::Error),

    /// Something very unexpected happened with time itself.
    #[error("system time error")]
    TimeError(#[from] time::SystemTimeError),
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
            .duration_since(time::UNIX_EPOCH)?
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
/// an installation access token.
#[derive(Debug, Deserialize, Eq, PartialEq)]
struct RawInstallationAccessToken {
    token: String,
    expires_at: DateTime<Utc>,
}

/// Use the app private key to generate a JWT and use the JWT to get
/// an installation access token.
///
/// Reference:
/// developer.github.com/apps/building-github-apps/authenticating-with-github-apps
fn get_installation_token(
    client: &reqwest::blocking::Client,
    params: &GithubAuthParams,
) -> Result<RawInstallationAccessToken, AuthError> {
    let claims = JwtClaims::new(params)?;
    let header = jsonwebtoken::Header {
        alg: jsonwebtoken::Algorithm::RS256,
        ..Default::default()
    };
    let private_key =
        jsonwebtoken::EncodingKey::from_rsa_pem(&params.private_key)?;
    let token = jsonwebtoken::encode(&header, &claims, &private_key)?;

    let url = format!(
        "https://api.github.com/app/installations/{}/access_tokens",
        params.installation_id
    );
    Ok(client
        .post(&url)
        .bearer_auth(token)
        .header("Accept", MACHINE_MAN_PREVIEW)
        .send()?
        .error_for_status()?
        .json()?)
}

/// An installation access token is the primary method for
/// authenticating with the GitHub API as an application.
pub struct InstallationAccessToken {
    /// The [`reqwest::blocking::Client`] used to periodically refresh
    /// the token.
    ///
    /// This is made public so that users of the library can re-use
    /// this client for sending requests, but this is not required.
    pub client: reqwest::blocking::Client,

    /// This time is subtracted from `expires_at` to make it less
    /// likely that the token goes out of date just as a request is
    /// sent.
    pub refresh_safety_margin: Duration,

    token: String,
    expires_at: DateTime<Utc>,
    params: GithubAuthParams,
}

impl InstallationAccessToken {
    /// Fetch an installation access token using the provided
    /// authentication parameters.
    pub fn new(
        params: GithubAuthParams,
    ) -> Result<InstallationAccessToken, AuthError> {
        let client = reqwest::blocking::Client::builder()
            .user_agent(&params.user_agent)
            .build()?;
        let raw = get_installation_token(&client, &params)?;
        Ok(InstallationAccessToken {
            client,
            token: raw.token,
            expires_at: raw.expires_at,
            params,
            refresh_safety_margin: Duration::minutes(1),
        })
    }

    /// Get an HTTP authentication header for the installation access
    /// token.
    ///
    /// This method is mutable because the installation access token
    /// must be periodically refreshed.
    pub fn header(&mut self) -> Result<HeaderMap, AuthError> {
        self.refresh()?;
        let mut headers = HeaderMap::new();
        let val = format!("token {}", self.token);
        headers.insert("Authorization", val.parse()?);
        Ok(headers)
    }

    fn needs_refresh(&self) -> bool {
        let expires_at = self.expires_at - self.refresh_safety_margin;
        expires_at <= Utc::now()
    }

    fn refresh(&mut self) -> Result<(), AuthError> {
        if self.needs_refresh() {
            info!("refreshing installation token");
            let raw = get_installation_token(&self.client, &self.params)?;
            self.token = raw.token;
            self.expires_at = raw.expires_at;
        }
        Ok(())
    }
}

/// Input parameters for authenticating as a GitHub app. This is used
/// to get an installation access token.
#[derive(Clone, Default)]
pub struct GithubAuthParams {
    /// User agent set for all requests to GitHub. The API requires
    /// that a user agent is set:
    /// <https://docs.github.com/en/rest/overview/resources-in-the-rest-api#user-agent-required>
    ///
    /// They "request that you use your GitHub username, or the name
    /// of your application".
    pub user_agent: String,

    /// Private key used to sign access token requests. You can
    /// generate a private key at the bottom of the application's
    /// settings page.
    pub private_key: Vec<u8>,

    /// GitHub application installation ID. To find this value you can
    /// look at the app installation's configuration URL.
    ///
    /// - For organizations this is on the "Installed GitHub Apps"
    ///   page in your organization's settings page.
    ///
    /// - For personal accounts, go to the "Applications" page and
    ///   select the "Installed GitHub Apps" tab.
    ///
    /// The installation ID will be the final component of the path,
    /// for example "1216616" is the installation ID for
    /// "github.com/organizations/mycoolorg/settings/installations/1216616".
    pub installation_id: u64,

    /// GitHub application ID. You can find this in the application
    /// settings page on GitHub under "App ID".
    pub app_id: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_raw_installation_access_token_parse() {
        let resp = r#"{
            "token": "v1.1f699f1069f60xxx",
            "expires_at": "2016-07-11T22:14:10Z"
            }"#;
        let token =
            serde_json::from_str::<RawInstallationAccessToken>(resp).unwrap();
        assert_eq!(
            token,
            RawInstallationAccessToken {
                token: "v1.1f699f1069f60xxx".into(),
                expires_at: Utc.ymd(2016, 7, 11).and_hms(22, 14, 10),
            }
        );
    }

    #[test]
    fn test_needs_refresh() {
        use std::thread::sleep;
        let mut token = InstallationAccessToken {
            client: reqwest::blocking::Client::new(),
            token: "myToken".into(),
            expires_at: Utc::now() + Duration::seconds(2),
            params: GithubAuthParams::default(),
            refresh_safety_margin: Duration::seconds(0),
        };
        assert!(!token.needs_refresh());
        sleep(Duration::milliseconds(1500).to_std().unwrap());
        assert!(!token.needs_refresh());
        token.refresh_safety_margin = Duration::seconds(1);
        assert!(token.needs_refresh());
    }
}
