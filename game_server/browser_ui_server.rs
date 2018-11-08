extern crate rouille;
use self::rouille::{Response, extension_to_mime};

#[derive(RustEmbed)]
#[folder = "game_browser/dist/"]
struct Asset;

pub fn start_browser_ui_server(version: &'static str, network_config: ::init::NetworkConfig) {
    rouille::start_server(network_config.serve_host_port.clone(), move |request| {
        if request.raw_url() == "/" {
            println!("{:?} loaded page", request.remote_addr());

            let template = ::std::str::from_utf8(
                &Asset::get("index.html").expect("index.html should exist as asset"),
            ).unwrap()
            .to_owned();

            let rendered = template
                .replace("CB_VERSION", version.trim())
                .replace(
                    "CB_BATCH_MESSAGE_BYTES",
                    &format!("{}", network_config.batch_msg_bytes),
                ).replace(
                    "CB_ACCEPTABLE_TURN_DISTANCE",
                    &format!("{}", network_config.ok_turn_dist),
                ).replace(
                    "CB_SKIP_TURNS_PER_TURN_AHEAD",
                    &format!("{}", network_config.skip_ratio),
                );

            Response::html(rendered)
        } else if let Some(asset) = Asset::get(&request.url()[1..]) {
            Response::from_data(
                if request.url().ends_with(".wasm") {
                    "application/wasm"
                } else {
                    extension_to_mime(request.url().split('.').last().unwrap_or(""))
                },
                asset,
            )
        } else {
            Response::html(format!("404 error. Not found: {}", request.url())).with_status_code(404)
        }
    });
}
