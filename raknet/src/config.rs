use rand;

pub struct Config {
    pub ping_response: String,
    pub guid: u64,        
}

impl Default for Config {
    fn default() -> Config {
        Config {
            ping_response: String::new(),
            guid: rand::random(),
        }
    }
}