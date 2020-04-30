# Making a release

1. Verify that `cargo test` passes.
2. Update the `version` field in `Cargo.toml`
3. Commit the change: `git commit -am 'Bump version'`
4. Push the changes: `git push`
5. Wait for the GitHub CI to pass. Note that it runs a test that is
   not run locally.
6. Create a git tag: `git tag vX.Y.Z`
7. Push the tag: `git push --tags`
8. Publish to crates.io: `cargo publish`
