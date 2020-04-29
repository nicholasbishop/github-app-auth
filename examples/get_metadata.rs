use github_app_auth::{GithubAuthParams, InstallationAccessToken};
use std::{env, os::unix::ffi::OsStrExt};

type BoxError = Box<dyn std::error::Error>;

fn get_var_bytes(name: &str) -> Result<Vec<u8>, BoxError> {
    let value = env::var_os(name).ok_or(format!("env var {} not set", name))?;
    Ok(value.as_bytes().into())
}

// This example requires read-only access to the repository metadata.
fn main() -> Result<(), BoxError> {
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

    let resp = token
        .client
        .get(&format!("https://api.github.com/repos/{}/license", repo))
        .headers(token.header()?)
        .send()?
        .error_for_status()?
        .json()?;

    println!("response: {:?}", resp);

    Ok(())
}
