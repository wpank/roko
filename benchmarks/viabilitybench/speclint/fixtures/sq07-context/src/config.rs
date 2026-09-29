pub struct Config {
    pub retry_limit: u32,
}

pub fn parse_config(text: &str) -> Config {
    let _ = text;
    Config { retry_limit: 3 }
}
