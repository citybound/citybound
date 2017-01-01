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
* Windows: `rustup override add nightly-2016-10-28-x86_64-pc-windows-gnu`
* MacOS: `rustup override add nightly-2016-10-28-x86_64-apple-darwin`
* Linux: `rustup override add nightly-2016-10-28-x86_64-unknown-linux-gnu`
* `cargo run --release` (Debug mode is generally too slow to interact with)

# License

Citybound is licensed under AGPL (see [LICENSE.txt](LICENSE.txt)), for interest in commercial licenses please contact me.

# Contribution

The code is in a pretty messy state after a rushed first release, but will become much more modular and well-documented over time.
Issues are categorized into levels of difficulty amongst other properties, but the ones flagged with "Assistance Welcome" are most likely to be tackleable by outside contributors.
Pull requests of any kind are welcome, but there is no defined process or acceptance criteria yet, we'll just figure it out along the way.

Unless explicitly stated otherwise, all contributions are assumed to be licensed under AGPL as well.
