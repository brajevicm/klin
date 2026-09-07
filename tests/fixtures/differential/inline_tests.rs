pub fn read(name: &str) -> String {
    std::fs::read_to_string(name).unwrap()
}

pub fn parse(text: &str) -> u32 {
    text.trim().parse().expect("a number")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_parses() {
        assert_eq!(parse(" 3 "), 3);
        let _ = read("missing").unwrap();
    }

    #[test]
    #[ignore]
    fn it_waits() {
        let _ = "x".parse::<u32>().unwrap();
    }
}
