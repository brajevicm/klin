#[expect(clippy::needless_return)]
pub fn port() -> u16 {
    return 8080;
}
