use log::info;
use simplelog::*;
use std::fs::File;

fn main() {
    CombinedLogger::init(
        vec![
            SimpleLogger::new(LevelFilter::Info, Config::default()),
            WriteLogger::new(LevelFilter::Info, Config::default(), File::create("bedroxide.log").unwrap()),
        ]
    ).unwrap();

    info!("Bedroxide started!");
}
