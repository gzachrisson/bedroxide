use crate::{
    config::Config,
    socket::DatagramSocket,
};

pub struct Communicator<T: DatagramSocket> {
    config: Config,
    socket: T,
}

impl<T: DatagramSocket> Communicator<T> {
    pub fn new(socket: T, config: Config) -> Self {
        Communicator {
            config,
            socket,
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn socket(&mut self) -> &mut T {
        &mut self.socket
    }
}