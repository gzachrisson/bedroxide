use rand;

pub struct Config {
    /// A unique (random) identifier that identifies this peer in
    /// connections with other peers.
    pub guid: u64,

    /// The maximum number of incoming connections, thus not initiated
    /// by this peer. If set to 0 the peer will only act as a client. 
    pub max_incoming_connections: usize,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            guid: rand::random(),
            max_incoming_connections: 50,
        }
    }
}