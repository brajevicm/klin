pub fn post_json(path: &str, body: &str) -> crate::Result<String> {
    crate::http::transport::send("POST", path, body, 3)
}
