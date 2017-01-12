## Just reporting a bug?

Make sure you read: [how to report bugs](https://github.com/citybound/citybound/wiki/How-to-report-bugs)

# Plan to contribute code?

## Make sure to <a href="https://www.clahub.com/agreements/citybound/citybound">sign the Contributor License Agreement</a>.

## Compiling Citybound from source 

Currently Citybound is built with Rust `nightly-2017-01-08`

Recommended setup:
* install [rustup](https://rustup.rs/)
* `git clone https://github.com/citybound/citybound.git`
* `cd citybound`
* Windows: `rustup override add nightly-2017-01-08-x86_64-pc-windows-gnu`
* MacOS: `rustup override add nightly-2017-01-08-x86_64-apple-darwin`
* Linux: `rustup override add nightly-2017-01-08-x86_64-unknown-linux-gnu`
* `cargo run --release` (Debug mode is generally too slow to interact with)

## Conforming to style

* install rustfmt: `cargo install rustfmt`
* run rustfmt on the whole repo:
  `rustfmt --write-mode=overwrite ./src/main.rs ./lib/*/src/lib.rs`
  (using default settings)


## Getting the recommended dev environment

* Install [Visual Studio Code](https://code.visualstudio.com)
* Install [the RustyCode Extension and its dependencies](https://marketplace.visualstudio.com/items?itemName=saviorisdead.RustyCode)
* Make sure to set up `"rust.cargoHomePath"`, `"rust.racerPath"` and `"rust.rustLangSrcPath"` in the VS Code settings
* For debugging (Linux/MacOS): Install the [LLDB Debugger Extension](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)
* Now everything should just work, since configuration is part of the repo in `.vscode`! (fingers crossed)

## State of the code & organization

The code is in a pretty messy state after a rushed first release, but will become much more modular and well-documented over time.
Issues are categorized into levels of difficulty amongst other properties, but the ones flagged with "Assistance Welcome" are most likely to be tackleable by outside contributors.
Pull requests of any kind are welcome, but there is no defined process or acceptance criteria yet, we'll just figure it out along the way.