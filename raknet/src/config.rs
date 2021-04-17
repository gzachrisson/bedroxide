use rand;

pub struct Config {
    pub guid: u64,        
}

impl Default for Config {
    fn default() -> Config {
        Config {
            guid: rand::random(),
        }
    }
}