use github_app_auth::{GithubAuthParams, InstallationAccessToken};
use serde::Deserialize;
use std::{env, os::unix::ffi::OsStrExt};

type BoxError = Box<dyn std::error::Error>;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct Secret {
    name: String,
}

impl Secret {
    fn new(name: &str) -> Secret {
        Secret { name: name.into() }
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct SecretsResponse {
    total_count: u32,
    secrets: Vec<Secret>,
}

fn get_var_bytes(name: &str) -> Result<Vec<u8>, BoxError> {
    let value = env::var_os(name).ok_or(format!("env var {} not set", name))?;
    Ok(value.as_bytes().into())
}

// This test requires read-only access to the repository secrets. It
// is ignored by default, but the github CI runner enables ignored
// tests.
#[test]
#[ignore]
fn get_secrets() -> Result<(), BoxError> {
    let private_key = get_var_bytes("TEST_PRIVATE_KEY")?;
    let app_id = env::var("TEST_APP_ID")?.parse::<u64>()?;
    let installation_id = env::var("TEST_INSTALLATION_ID")?.parse::<u64>()?;

    // Format: owner/repo
    let repo = env::var("GITHUB_REPOSITORY")?;

    let mut token = InstallationAccessToken::new(GithubAuthParams {
        user_agent: "github-app-auth-example".into(),
        private_key,
        app_id,
        installation_id,
    })?;

    let resp: SecretsResponse = token
        .client
        .get(&format!(
            "https://api.github.com/repos/{}/actions/secrets",
            repo
        ))
        .headers(token.header()?)
        .send()?
        .error_for_status()?
        .json()?;

    assert_eq!(
        resp,
        SecretsResponse {
            total_count: 3,
            secrets: vec![
                Secret::new("TEST_APP_ID"),
                Secret::new("TEST_INSTALLATION_ID"),
                Secret::new("TEST_PRIVATE_KEY"),
            ]
        }
    );

    println!("response: {:?}", resp);

    Ok(())
}
