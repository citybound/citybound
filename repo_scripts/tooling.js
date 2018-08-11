const util = require('util');
const { execSync, spawnSync } = require('child_process');

const NIGHTLY_VERSION = "nightly-2018-08-06";
let quiet = process.argv[2] == "-q";

let rustupV;
try {
    rustupV = execSync('rustup --version', { encoding: 'utf8' });
} catch (e) {
    rustupV = undefined
}

if (rustupV && rustupV.startsWith("rustup")) {
    !quiet && console.log("Rustup installed ✅");
} else {
    console.log("Rustup missing! 🛑");
    console.log("Please install from https://rustup.rs");
    process.exit(1);
}


let rustupShow = execSync('rustup show', { encoding: 'utf8' });

let activeToolchains = rustupShow.split(/^active toolchain$/m)[1];

if (activeToolchains && activeToolchains.includes(NIGHTLY_VERSION)) {
    !quiet && console.log("Correct rust nightly set up ✅");
    correctToolchain = true;
} else {
    console.log("Wrong version of rust set up ⚠️");
    activeToolchains && console.log(activeToolchains.split(/\n/g).map(s => " | " + s).join("\n"));
    console.log("🔧 Overriding with correct nightly (only for this directory)...");
    const nightlySuffix = process.platform === "win32"
        ? "-x86_64-pc-windows-msvc"
        : (process.platform === "darwin"
            ? "-x86_64-apple-darwin"
            : "-x86_64-unknown-linux-gnu");
    const fullVersion = NIGHTLY_VERSION + nightlySuffix;
    console.log("> rustup override set " + fullVersion);
    spawnSync("rustup", ["override", "set", fullVersion], { stdio: 'inherit' });
    quiet = false;

    let rustupShow2 = execSync('rustup show', { encoding: 'utf8' });

    let activeToolchains2 = rustupShow2.split(/^active toolchain$/m)[1];
    if (activeToolchains2 && activeToolchains2.includes(NIGHTLY_VERSION)) {
        !quiet && console.log("Correct rust nightly set up ✅");
    } else {
        console.log("Failed to install correct toolchain 🛑");
        process.exit(1);
    }
}

!quiet && console.log("🔧 Ensuring linting tools are installed...");
spawnSync("rustup", ["component", "add", "rustfmt-preview", "--toolchain", NIGHTLY_VERSION],
    { stdio: quiet ? 'ignore' : 'inherit' }
);
spawnSync("rustup", ["component", "add", "clippy-preview", "--toolchain", NIGHTLY_VERSION],
    { stdio: quiet ? 'ignore' : 'inherit' }
);
!quiet && console.log("Linting tools set up ✅");