pub struct Connection {
    guid: u64,
    is_incoming: bool,
    mtu: u16,
    pub state: ConnectionState,
}

impl Connection {
    pub fn new(guid: u64, is_incoming: bool, mtu: u16) -> Connection {
        Connection {
            guid,
            is_incoming,
            mtu,
            state: ConnectionState::UnverifiedSender,
        }
    }

    /// Returns the GUID of the remote peer.
    pub fn guid(&self) -> u64 {
        self.guid
    }

    /// Returns the agreed MTU for this connection.
    pub fn mtu(&self) -> u16 {
        self.mtu
    }

    /// Returns true if the connection was initiated
    /// by a remote peer.
    pub fn is_incoming(&self) -> bool {
        self.is_incoming
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum ConnectionState {
    // NoAction,
    // DisconnectAsap,
    // DisconnectAsapSilently,
    // DisconnectOnNoAck,
    // RequestedConnection,
    // HandlingConnectionRequest,
    UnverifiedSender,
    Connected,
}