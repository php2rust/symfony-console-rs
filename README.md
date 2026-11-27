# symfony-console-rs

Rust ports of [symfony/console](https://github.com/symfony/console) (a
subset) and [symfony/string](https://github.com/symfony/string).
Unofficial, not affiliated with or endorsed by Symfony. MIT, as upstream.

Part of the php2rust project. The crates came from
[symfony-rs](https://github.com/php2rust/symfony-rs) `crates/console` and
`crates/string`, with their git history.

| Crate | Upstream | Contents |
|---|---|---|
| [php2rust-symfony-console](crates/php2rust-symfony-console/README.md) | symfony/console v8.1.9 | Formatter, input, output, commands, the application's registry, tables, SymfonyStyle, the testers |
| [php2rust-symfony-string](crates/php2rust-symfony-string/README.md) | symfony/string v8.1.10 | ByteString, CodePointString, UnicodeString, LazyString, the inflectors |

Dependencies: php2rust/compatibility-tools-rs (PHP values and PCRE) and
php2rust/symfony-deprecation-contracts-rs. This repository does not depend
on symfony-rs.

## Use

The crates are not on crates.io. Use git dependencies pinned to a rev:

```toml
[dependencies]
symfony-console = { package = "php2rust-symfony-console", git = "https://github.com/php2rust/symfony-console-rs", rev = "<commit>" }
symfony-string = { package = "php2rust-symfony-string", git = "https://github.com/php2rust/symfony-console-rs", rev = "<commit>" }
```

The library names stay `symfony_console` and `symfony_string`.

## Checks

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
