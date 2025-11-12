pub struct Config {
    pub port: i32,
}

const CONFIG: Config = Config { port: 5215 };

pub fn get_config() -> Config {
    return CONFIG;
}
