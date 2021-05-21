use rand;

pub struct Config {
    /// A unique (random) identifier that identifies this peer in
    /// connections with other peers.
    pub guid: u64,

    /// The maximum number of incoming connections, thus not initiated
    /// by this peer. If set to 0 the peer will only act as a client. 
    pub max_incoming_connections: usize,

    /// The time in milliseconds that a remote peer has to send a
    /// connection request before the connection get dropped.
    ///
    /// The time is measured from when this peer has sent
    /// an "open connection reply 2" until we receive a "connection request".
    pub incoming_connection_timeout_in_ms: u128,

    /// The time in milliseconds before a connection is considered dead
    /// if no datagrams have been received when this peer has sent packets
    /// that are awaiting acks.
    pub ack_timeout_in_ms: u128,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            guid: rand::random(),
            max_incoming_connections: 50,
            incoming_connection_timeout_in_ms: 10000,
            ack_timeout_in_ms: 10000,
        }
    }
}