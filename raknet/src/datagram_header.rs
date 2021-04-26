use std::option::Option;

use crate::{RakNetRead, RakNetWrite, ReadError, Result, SequenceNumber, u24};

pub enum DatagramHeader {
    Ack { data_arrival_rate: Option<f32> },
    Nack,
    Packet { is_packet_pair: bool, is_continuous_send: bool, needs_data_arrival_rate: bool, datagram_number: SequenceNumber },
}

impl DatagramHeader {
    pub fn read(reader: &mut impl RakNetRead) -> Result<Self> {
        let bitflags = reader.read_u8()?;
        let is_valid = (bitflags & (1 << 7)) != 0;
        if !is_valid { return Err(ReadError::InvalidHeader.into()); }

        let is_ack = (bitflags & (1 << 6)) != 0;
        if is_ack {
            let has_data_arrival_rate = (bitflags & (1 << 5)) != 0;
            let data_arrival_rate = if has_data_arrival_rate {
                Some(reader.read_f32_be()?)
            } else {
                None
            };
            Ok(DatagramHeader::Ack { data_arrival_rate })
        } else {
            let is_nack = (bitflags & (1 << 5)) != 0;
            if is_nack {
                Ok(DatagramHeader::Nack)
            } else {
                let is_packet_pair = (bitflags & (1 << 4)) != 0;
                let is_continuous_send  = (bitflags & (1 << 3)) != 0; 
                let needs_data_arrival_rate = (bitflags & (1 << 2)) != 0;
                let datagram_number = SequenceNumber::from(reader.read_u24()?);
                Ok(DatagramHeader::Packet { is_packet_pair, is_continuous_send, needs_data_arrival_rate, datagram_number })
            }
        }
    }

    pub fn _write(&self, writer: &mut dyn RakNetWrite) -> Result<()> {
        // Bit 7 = "isValid"
        let mut bitflags: u8 = 1 << 7;
        match self {
            DatagramHeader::Ack { data_arrival_rate } => {
                // Bit 6 = "isAck"
                bitflags |= 1 << 6;
                if let Some(data_arrival_rate) = data_arrival_rate {    
                    // Bit 5 = "hasBAndAS" (data arrival rate)              
                    bitflags |= 1 << 5;
                    writer.write_u8(bitflags)?;
                    writer.write_f32_be(*data_arrival_rate)?;
                } else {

                }
            },
            DatagramHeader::Nack => {
                // Bit 5 = "isNack"
                bitflags |= 1 << 5;
                writer.write_u8(bitflags)?;
            },
            DatagramHeader::Packet { is_packet_pair, is_continuous_send, needs_data_arrival_rate, datagram_number } => {
                bitflags |= if *is_packet_pair { 1 << 4 } else { 0 };
                bitflags |= if *is_continuous_send { 1 << 3 } else { 0 };
                bitflags |= if *needs_data_arrival_rate { 1 << 2 } else { 0 };
                writer.write_u8(bitflags)?;
                writer.write_u24(u24::from(datagram_number))?;
            },
        }
        Ok(())
    }
}
