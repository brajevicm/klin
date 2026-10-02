pub mod log;

pub fn start(port: u16) {
    log::log(&format!("listening on {port}"));
}

pub fn stop() {
    std::println!("stopping");
}
