use github_app_auth::{GithubAuthParams, InstallationAccessToken};
use serde::Deserialize;
use std::{env, ffi::OsStr, os::unix::ffi::OsStrExt};

type BoxError = Box<dyn std::error::Error>;

#[derive(Deserialize)]
struct License {
    key: String,
}

#[derive(Deserialize)]
struct LicenseResponse {
    license: License,
}

fn get_var_bytes(name: &str) -> Result<Vec<u8>, BoxError> {
    let value = env::var_os(name).ok_or(format!("env var {} not set", name))?;
    Ok(value.as_bytes().into())
}

fn is_running_in_ci() -> bool {
    env::var_os("CI") == Some(OsStr::from_bytes(b"true").into())
}

// This test requires read-only access to the repository metadata.
#[test]
fn get_metadata() -> Result<(), BoxError> {
    if !is_running_in_ci() {
        return Ok(());
    }

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

    let resp: LicenseResponse = token
        .client
        .get(&format!("https://api.github.com/repos/{}/license", repo))
        .headers(token.header()?)
        .send()?
        .error_for_status()?
        .json()?;

    assert_eq!(resp, LicenseResponse {
        license: License {
            key: "apache-2.0",
        },
    });

    println!("response: {:?}", resp);

    Ok(())
}
