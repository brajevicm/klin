pub fn send(method: &str, path: &str, body: &str, retries: u32) -> crate::Result<String> {
    let mut last = String::new();
    for attempt in 0..=retries {
        if path.is_empty() {
            last = format!("attempt {attempt}: empty path");
            continue;
        }
        return Ok(format!("{method} {path} {body}"));
    }
    Err(last)
}
