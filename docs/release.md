# Making a release

1. Update the `version` field in `Cargo.toml`
2. Commit the change: `git commit -am 'Bump version'`
3. Create a git tag: `git tag vX.Y.Z`
4. Push the changes: `git push && git push --tags`
5. Publish to crates.io: `cargo publish`
