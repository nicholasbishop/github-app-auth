# github-app-auth

[![crates.io](https://img.shields.io/crates/v/github-app-auth.svg)](https://crates.io/crates/github-app-auth)
[![Documentation](https://docs.rs/github-app-auth/badge.svg)](https://docs.rs/github-app-auth)

This is a small library for authenticating with the GitHub API as a
GitHub app.

Documentation on the overall flow:
https://developer.github.com/apps/building-github-apps/authenticating-with-github-apps

## Making a release

1. Update the `version` field in `Cargo.toml`
2. Commit the change: `git commit -am 'Bump version'`
3. Create a git tag: `git tag vX.Y.Z`
4. Push the changes: `git push && git push --tags`
5. Publish to crates.io: `cargo publish`
