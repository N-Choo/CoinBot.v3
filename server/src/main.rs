use log::info;
use server::logger::init_logger;

fn main() {
    init_logger();
    info!("Starting server...");
}
