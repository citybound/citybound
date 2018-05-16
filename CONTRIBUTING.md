# Overview

* [Reporting bugs](#reporting-bugs)
* [Contributing to the Design Doc](#contributing-to-the-design-doc)
* [Compiling Citybound yourself](#compiling-citybound-yourself)
* [Contributing to the code](#contributing-code)

# Reporting Bugs

If the game stops working, it usually displays a red `SIMULATION BROKE :(` message.

More details should be visible in the editor that automatically opens, or in a `cb_last_error.txt` file in your system's temporary directory.

**First look if your issue has already been reported:**
* [as a Bug](https://github.com/citybound/citybound/issues?utf8=✓&q=is%3Aissue%20label%3A%22P%20Bug%22%20)
* [as Not Fun](https://github.com/citybound/citybound/issues?utf8=✓&q=is%3Aissue%20label%3A%22P%20Not%20Fun%22%20)
* [as an Annoyance](https://github.com/citybound/citybound/issues?q=is%3Aissue+label%3A%22P+Annoyance%22)

If not, [create a new issue](https://github.com/aeickhoff/citybound/issues/new).

* Please provide details
    * what platform you're on
    * what you were trying to do or what you expected to happen
    * what actually happened
    * the detailed error information
* Ideally: If the game is still running, take one or several screenshots (camera controls continue to work in many cases)
* Perfect: Provide precise instructions on how to reproduce the problem (if possible)

# Contributing to the Design Doc

The [Design Doc](game/README.md) outlines the philosophy and decisions that I follow when implementing Citybound.

You can make suggestions of every kind:

* typos/formatting/clarification
* feature requests
* complete system design proposals

**[All existing design proposals](https://github.com/citybound/citybound/pulls?utf8=✓&q=is%3Apr%20label%3A%22DESIGN%20PROPOSAL%22%20)**

* If you have some rough ideas, it is probably best to discuss them in the [official community](https://reddit.com/r/Citybound) first, where people can give you first feedback and point you to existing relevant Design Proposals
* If you have an original and detailed proposal, start editing relevant documents of the Design Doc, or add new ones
  * [Small tutorial on how to do that in the GitHub Web interface](https://help.github.com/articles/editing-files-in-another-user-s-repository/)
  * Please give your Pull Request the DESIGN PROPOSAL label
  * Invite people from the [official community](https://reddit.com/r/Citybound) or authors of other Design Proposals to comment on and suggest improvements/clarifications to your pull request
  * I will take a look at your pull request and give it a detailed review if it meets minimal quality standards
  * We will iterate on it together, a process in which you have the opportunity to explain your motivation and potentially convice me to do things in the way you suggested
  * **Make sure to <a href="https://www.clahub.com/agreements/citybound/citybound">sign the Contributor License Agreement</a>.**
  * In the end, we either
     * fully agree with your proposal and merge it into the official design doc, or:
     * we identify a compromise of a subset of the proposed changes and merge that, or:
     * if our disagreement is too large, the pull request gets closed, but with a thorough explanation from my side

## Compiling Citybound yourself

Currently Citybound is built with Rust `nightly-2018-05-07'

**If you want a working version of Citybound,** compile a commit that corresponds to a [release](https://github.com/citybound/citybound/releases), since master might temporarily break or represents work-in-progress state.

Recommended setup:
* Install [rustup](https://rustup.rs/) and [git](https://git-scm.com/)
* `git clone https://github.com/citybound/citybound.git`
* `cd citybound`
* Windows:
  * `rustup override add nightly-2018-05-07-x86_64-pc-windows-msvc`
  * Install the [Visual C++ 2015 Build Tools](http://landinghub.visualstudio.com/visual-cpp-build-tools), unless you have Visual Studio 2015
* MacOS:
  * `rustup override add nightly-2018-05-07-x86_64-apple-darwin`
* Linux:
  * `rustup override add nightly-2018-05-07-x86_64-unknown-linux-gnu`
  * `sudo apt install build-essential` (for Ubuntu)
* `cargo run --release` (Debug mode is generally too slow to interact with)

# Contributing Code

## Guidelines

* **Make sure to <a href="https://www.clahub.com/agreements/citybound/citybound">sign the Contributor License Agreement</a>.**
* **[Have a look at the documentation](http://citybound.github.io/citybound)**
* **Citybound uses trunk-based development,** meaning a very recent work-in-progress state of the code is always in the master branch. The repository owner usually commits directly to master, or uses short-lived feature branches. Contributors use the common fork/pull-request flow and everyone involved tries to get the changes into master as quickly as possible. The newest commits in master might sometimes be broken and not run.


## Getting the recommended dev environment

* Install [Visual Studio Code](https://code.visualstudio.com)
  * It's a (cross-platform + JS-based + rich plugin ecosystem) Editor like Atom, only snappier - (it also has nothing to do with Visual Studio)
  * Yes it's actually cool, because Microsoft has started to be cool.
* Install [the VSCode-Rust Extension](https://marketplace.visualstudio.com/items?itemName=kalitaalexey.vscode-rust)
  * Let it install everything it wants to
  * *If you are using Windows and have a space in your user name:*
    * Create a symbolic link to you user folder that doesn't contain a space
      * for example `C:\firstname` -> `C:\Users\Firstname Lastname`
    * Add the following user settings in VSCode
      * `"rust.cargoHomePath": "C:\\firstname\\.cargo"`,
      * `"rust.racerPath": "C:\\firstname\\.cargo\\bin\\racer.exe"`,
      * `"rust.rustLangSrcPath": "C:\\firstname\\.rustup\\toolchains\\nightly-2018-05-07-x86_64-pc-windows-msvc\\lib\\rustlib\\src\\rust\\src"`
  * Otherwise it "should just work"
* For debugging 
  * Linux/MacOS: 
    * Install the [LLDB Debugger Extension](https://marketplace.visualstudio.com/items?itemName=vadimcn.vscode-lldb)
  * Windows:
    * Install the [C/C++ Extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode.cpptools)
    * Select the "(Windows) Debug" Configuration when running the debug program in VS Code

* Now everything should just work! (fingers crossed)

## Conforming to style

* install rustfmt: `cargo install rustfmt --vers 0.9.0` **and please make sure to use the same version as noted here** (pinned now, but might change from time to time)
* run rustfmt on the whole repo:
  `rustfmt --write-mode=overwrite ./game/main.rs ./engine/*/src/lib.rs`
  (using default settings) - if there are any overlong lines it can't fix, please fix them manually.
* You should also **fix all clippy warnings** properly

## Have a question? Want to discuss something?

Join me and the other contributors in the [Gitter community for Citybound](https://gitter.im/citybound/Lobby) and ask/discuss away!
