const fs = require("fs");
fs.existsSync("dist") || fs.mkdirSync("dist");
fs.copyFileSync("target/wasm32-unknown-unknown/release/citybound_browser.wasm", "dist/citybound_browser.wasm");