# Agent rules

- Port upstream `symfony/console` and `symfony/string` behaviour exactly; check it against PHP (in docker only).
- No PHP source in this repository. Each module doc has an `//! Upstream:` line.
- No `unwrap`/`expect` on input, no `unsafe`. `publish = false`; no crates.io.
- Dependencies on other php2rust repositories: only compatibility-tools-rs and symfony-deprecation-contracts-rs, as git deps pinned to a rev. Never depend on symfony-rs. Never commit a `[patch]` section.
- Run the three checks in README.md before a merge. No CI workflows.
