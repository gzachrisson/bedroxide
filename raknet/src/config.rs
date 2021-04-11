use rand;

pub struct Config {
    pub ping_response: Vec<u8>,
    pub guid: u64,        
}

impl Default for Config {
    fn default() -> Config {
        Config {
            ping_response: Vec::new(),
            guid: rand::random(),
        }
    }
}