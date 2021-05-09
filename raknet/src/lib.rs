pub use crossbeam_channel as channel;

pub use self::{
    config::Config,
    error::{Error, Result, ReadError, WriteError},
    number::{DatagramSequenceNumber, OrderingChannelIndex, u24},
    packet::Packet,
    peer::{Peer, Command},
    peer_event::PeerEvent,
    reader::DataRead,
    writer::DataWrite,
};

mod acknowledgement;
mod communicator;
mod config;
mod connection;
mod connection_manager;
mod constants;
mod datagram_header;
mod error;
mod split_packet_handler;
mod internal_packet;
mod message_ids;
mod messages;
mod number;
mod offline_packet_handler;
mod packet;
mod peer;
mod peer_event;
mod reader;
mod socket;
mod utils;
mod writer;
