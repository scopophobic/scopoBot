# Contributing

Small, focused changes are welcome. Open an issue for larger design changes first. Mac and Windows are the primary desktop platforms; Linux terminal support should keep working.

Install stable Rust and your platform compiler tools, then run:

```sh
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Test desktop interaction on your platform for UI changes. Include your operating system, what changed, and validation results in a pull request. Keep sensor collection aggregate and local. Never collect application names, audio samples, screen contents, or device identifiers without an explicit product design discussion.

Character packs are JSON and PNG; see docs/character-packs.md. Bundled Glitch is generated raster animation inspired by the supplied references, not a skeletal rig. New assets should include their source and permission to distribute.
