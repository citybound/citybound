const fs = require("fs");
fs.existsSync("dist") || fs.mkdirSync("dist");
fs.copyFileSync("target/wasm32-unknown-unknown/release/cb_browser_ui.wasm", "dist/cb_browser_ui.wasm");