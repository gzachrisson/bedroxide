use crate::{
    constants::UDP_HEADER_SIZE,
    datagram_header::DatagramHeader,
    error::Result,
    internal_packet::InternalPacket,
    number::DatagramSequenceNumber,
    writer::DataWrite
};

#[derive(Debug)]
pub struct PacketDatagram {
    header: DatagramHeader,
    packets: Vec<InternalPacket>,
    payload_size: u16,
}

impl PacketDatagram {
    pub fn new(datagram_number: DatagramSequenceNumber) -> Self {
        // TODO: Perhaps set is_continuous_send for second datagram
        PacketDatagram {
            header: DatagramHeader::Packet {
                is_packet_pair: false,
                is_continuous_send: false,
                needs_data_arrival_rate: false,
                datagram_number,
            },
            packets: Vec::new(),
            payload_size: 0,
        }
    }

    pub fn push(&mut self, packet: InternalPacket) {
        self.payload_size = self.payload_size + packet.get_size_in_bytes();
        self.packets.push(packet);
    }

    pub fn write(&self, writer: &mut impl DataWrite) -> Result<()> {
        self.header.write(writer)?;
        for packet in self.packets.iter() {
            packet.write(writer)?;
        }
        Ok(())
    }

    pub fn has_room_for(&self, packet: &InternalPacket, mtu: u16) -> bool {
        let packet_size = packet.get_size_in_bytes();
        if self.payload_size + packet_size > Self::get_max_payload_size(mtu) {
            false
        } else {
            true
        }
    }

    pub fn get_max_payload_size(mtu: u16) -> u16 {
        // Datagram bitflags (u8) + datagram number (u24)
        let datagram_header_size = 1 + 3;
        mtu - UDP_HEADER_SIZE - datagram_header_size
    }

    pub fn is_empty(&self) -> bool {
        self.packets.is_empty()
    }

    pub fn into_packets(self) -> Vec<InternalPacket> {
        self.packets
    }
}