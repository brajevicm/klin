pub fn clean() {
    std::fs::remove_file("tmp").ok();
}
