## Just reporting a bug?

Make sure you read: [how to report bugs](https://github.com/citybound/citybound/wiki/How-to-report-bugs)

# Compiling Citybound from source 

Currently Citybound is built with Rust `nightly-2017-04-28`

Recommended setup:
* Install [rustup](https://rustup.rs/) and [git](https://git-scm.com/)
* `git clone https://github.com/citybound/citybound.git`
* `cd citybound`
* Windows:
  * `rustup override add nightly-2017-04-28-x86_64-pc-windows-msvc` **(new!)**
  * Install the [Visual C++ 2015 Build Tools](http://landinghub.visualstudio.com/visual-cpp-build-tools)
* MacOS:
  * `rustup override add nightly-2017-04-28-x86_64-apple-darwin`
* Linux:
  * `rustup override add nightly-2017-04-28-x86_64-unknown-linux-gnu`
* `cargo run --release` (Debug mode is generally too slow to interact with)

## Conforming to style

* install rustfmt: `cargo install rustfmt`
* run rustfmt on the whole repo:
  `rustfmt --write-mode=overwrite ./src/main.rs ./lib/*/src/lib.rs`
  (using default settings)


## Getting the recommended dev environment

* Install [Visual Studio Code](https://code.visualstudio.com)
  * It's a (cross-platform + JS-based + rich plugin ecosystem) Editor like Atom, only snappier - (it also has nothing to do with Visual Studio)
  * Yes it's actually cool, because Microsoft has started to be cool.
* Before the next step, set rustup's default toolchain to the same one you used above:
  * `rustup default nightly-XXXX-XXXX-XXXX` (see "Recommended Setup" above)
* Install [the VSCode-Rust Extension](https://marketplace.visualstudio.com/items?itemName=kalitaalexey.vscode-rust)
  * Follow its instructions to install "RLS" and allow it to update rustup if asked for.
  * After it's done, you can reset rustup's default toolchain again, if you want.
* For debugging (Linux/MacOS): Install the [LLDB Debugger Extension](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)
* Now everything should just work! (fingers crossed)

# Plan to contribute code?

## Make sure to <a href="https://www.clahub.com/agreements/citybound/citybound">sign the Contributor License Agreement</a>.

## [Have a look at the documentation](http://citybound.github.io/citybound)

## Have a question? Want to discuss something?

Join me and the other contributors in the [Gitter community for Citybound](https://gitter.im/citybound/Lobby) and ask/discuss away!

## State of the code & organization

The code is in a pretty messy state after a rushed first release, but will become much more modular and well-documented over time.
Issues are categorized into levels of difficulty amongst other properties, but the ones flagged with "Assistance Welcome" are most likely to be tackleable by outside contributors.
Pull requests of any kind are welcome, but there is no defined process or acceptance criteria yet, we'll just figure it out along the way.
