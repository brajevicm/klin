pub fn or_empty<T, E>(result: Result<Vec<T>, E>) -> Vec<T> {
    match result {
        Ok(items) => items,
        Err(_) => Vec::new(),
    }
}
