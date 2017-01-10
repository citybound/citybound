# Citybound

Citybound is an independently developed city building game, available open source and funded though Patreon.
Find out more about Citybound [on the wiki](https://github.com/aeickhoff/citybound/wiki).

# [Latest binary releases](https://github.com/aeickhoff/citybound/releases)

# Building from source

Currently Citybound is only tested with Rust `nightly-2016-10-28`

Recommended setup:
* install [rustup](https://rustup.rs/)
* `git clone https://github.com/citybound/citybound.git`
* `cd citybound`
* Windows: `rustup override add nightly-2017-01-08-x86_64-pc-windows-gnu`
* MacOS: `rustup override add nightly-2017-01-08-x86_64-apple-darwin`
* Linux: `rustup override add nightly-2017-01-08-x86_64-unknown-linux-gnu`
* `cargo run --release` (Debug mode is generally too slow to interact with)

# License

Citybound is licensed under AGPL (see [LICENSE.txt](LICENSE.txt)), for interest in commercial licenses please contact me.

# [Contributing](CONTRIBUTING.md)
