mod commands;
mod config;
mod server;

use server::server as web_server;

fn main() {
    let cfg = config::get_config();
    web_server::start_server(cfg.port);
}
