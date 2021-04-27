use std::option::Option;

use crate::{RakNetRead, RakNetWrite, ReadError, Result, DatagramSequenceNumber, u24};

#[derive(Debug, PartialEq)]
pub enum DatagramHeader {    
    Ack { data_arrival_rate: Option<f32> },
    Nack,
    Packet { is_packet_pair: bool, is_continuous_send: bool, needs_data_arrival_rate: bool, datagram_number: DatagramSequenceNumber },
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
                let datagram_number = DatagramSequenceNumber::from(reader.read_u24()?);
                Ok(DatagramHeader::Packet { is_packet_pair, is_continuous_send, needs_data_arrival_rate, datagram_number })
            }
        }
    }

    #[allow(dead_code)]
    pub fn write(&self, writer: &mut dyn RakNetWrite) -> Result<()> {
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
                    writer.write_u8(bitflags)?;
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

#[cfg(test)]
mod tests {
    use std::{convert::TryFrom, io::Cursor, matches};
    
    use crate::{datagram_header::DatagramHeader, DatagramSequenceNumber};

    #[test]
    fn read_ack_header_with_data_arrival_rate() {
        // Arrange
        let payload = [0b1110_0000u8, 0x40, 0xa0, 0x00, 0x00];
        let mut reader = Cursor::new(payload);

        // Act
        let header = DatagramHeader::read(&mut reader).expect("Couldn't read header");

        // Assert
        assert!(matches!(header, DatagramHeader::Ack { data_arrival_rate: Some(rate) } if rate == 5.0));
    }

    #[test]
    fn read_ack_header_without_data_arrival_rate() {
        // Arrange
        let payload = [0b1100_0000u8];
        let mut reader = Cursor::new(payload);

        // Act
        let header = DatagramHeader::read(&mut reader).expect("Couldn't read header");

        // Assert
        assert!(matches!(header, DatagramHeader::Ack { data_arrival_rate: None }));
    }

    #[test]
    fn read_nack_header() {
        // Arrange
        let payload = [0b1010_0000u8];
        let mut reader = Cursor::new(payload);

        // Act
        let header = DatagramHeader::read(&mut reader).expect("Couldn't read header");

        // Assert
        assert!(matches!(header, DatagramHeader::Nack));
    }

    #[test]
    fn read_packet_header() {
        // Arrange
        let payload = [0b1000_0000u8, 0x56, 0x34, 0x12];
        let mut reader = Cursor::new(payload);

        // Act
        let header = DatagramHeader::read(&mut reader).expect("Couldn't read header");

        // Assert
        let expected_datagram_number = DatagramSequenceNumber::try_from(0x123456u32).unwrap();
        assert!(matches!(header, DatagramHeader::Packet { is_packet_pair, is_continuous_send, needs_data_arrival_rate, datagram_number }
            if !is_packet_pair && !is_continuous_send && !needs_data_arrival_rate && datagram_number == expected_datagram_number));
    }

    #[test]
    fn read_packet_header_packet_pair() {
        // Arrange
        let payload = [0b1001_0000u8, 0x56, 0x34, 0x12];
        let mut reader = Cursor::new(payload);

        // Act
        let header = DatagramHeader::read(&mut reader).expect("Couldn't read header");

        // Assert
        let expected_datagram_number = DatagramSequenceNumber::try_from(0x123456u32).unwrap();
        assert!(matches!(header, DatagramHeader::Packet { is_packet_pair, is_continuous_send, needs_data_arrival_rate, datagram_number }
            if is_packet_pair && !is_continuous_send && !needs_data_arrival_rate && datagram_number == expected_datagram_number));
    }    

    #[test]
    fn read_packet_header_continuous_send() {
        // Arrange
        let payload = [0b1000_1000u8, 0x56, 0x34, 0x12];
        let mut reader = Cursor::new(payload);

        // Act
        let header = DatagramHeader::read(&mut reader).expect("Couldn't read header");

        // Assert
        let expected_datagram_number = DatagramSequenceNumber::try_from(0x123456u32).unwrap();
        assert!(matches!(header, DatagramHeader::Packet { is_packet_pair, is_continuous_send, needs_data_arrival_rate, datagram_number }
            if !is_packet_pair && is_continuous_send && !needs_data_arrival_rate && datagram_number == expected_datagram_number));
    }        

    #[test]
    fn read_packet_header_needs_data_arrival_rate() {
        // Arrange
        let payload = [0b1000_0100u8, 0x56, 0x34, 0x12];
        let mut reader = Cursor::new(payload);

        // Act
        let header = DatagramHeader::read(&mut reader).expect("Couldn't read header");

        // Assert
        let expected_datagram_number = DatagramSequenceNumber::try_from(0x123456u32).unwrap();
        assert!(matches!(header, DatagramHeader::Packet { is_packet_pair, is_continuous_send, needs_data_arrival_rate, datagram_number }
            if !is_packet_pair && !is_continuous_send && needs_data_arrival_rate && datagram_number == expected_datagram_number));
    }  

    #[test]
    fn write_ack_header_with_data_arrival_rate() {
        // Arrange
        let header = DatagramHeader::Ack { data_arrival_rate: Some(5.0) };
        let mut payload = Vec::new();

        // Act
        header.write(&mut payload).expect("Couldn't write header");

        // Assert
        assert_eq!(payload, vec![0b1110_0000u8, 0x40, 0xa0, 0x00, 0x00]);
    }

    #[test]
    fn write_ack_header_without_data_arrival_rate() {
        // Arrange
        let header = DatagramHeader::Ack { data_arrival_rate: None };
        let mut payload = Vec::new();

        // Act
        header.write(&mut payload).expect("Couldn't write header");

        // Assert
        assert_eq!(payload, vec![0b1100_0000u8]);
    }

    #[test]
    fn write_nack_header() {
        // Arrange
        let header = DatagramHeader::Nack;
        let mut payload = Vec::new();

        // Act
        header.write(&mut payload).expect("Couldn't write header");

        // Assert
        assert_eq!(payload, vec![0b1010_0000u8]);
    }

    #[test]
    fn write_packet_header() {
        // Arrange
        let header = DatagramHeader::Packet {
            is_packet_pair: false,
            is_continuous_send: false,
            needs_data_arrival_rate: false,
            datagram_number: DatagramSequenceNumber::try_from(0x123456u32).unwrap()
        };
        let mut payload = Vec::new();

        // Act
        header.write(&mut payload).expect("Couldn't write header");

        // Assert
        assert_eq!(payload, vec![0b1000_0000u8, 0x56, 0x34, 0x12]);
    }

    #[test]
    fn write_packet_header_packet_pair() {
        // Arrange
        let header = DatagramHeader::Packet {
            is_packet_pair: true,
            is_continuous_send: false,
            needs_data_arrival_rate: false,
            datagram_number: DatagramSequenceNumber::try_from(0x123456u32).unwrap()
        };
        let mut payload = Vec::new();

        // Act
        header.write(&mut payload).expect("Couldn't write header");

        // Assert
        assert_eq!(payload, vec![0b1001_0000u8, 0x56, 0x34, 0x12]);
    }    

    #[test]
    fn write_packet_header_continuous_send() {
        // Arrange
        let header = DatagramHeader::Packet {
            is_packet_pair: false,
            is_continuous_send: true,
            needs_data_arrival_rate: false,
            datagram_number: DatagramSequenceNumber::try_from(0x123456u32).unwrap()
        };
        let mut payload = Vec::new();

        // Act
        header.write(&mut payload).expect("Couldn't write header");

        // Assert
        assert_eq!(payload, vec![0b1000_1000u8, 0x56, 0x34, 0x12]);
    }        

    #[test]
    fn write_packet_header_needs_data_arrival_rate() {
        // Arrange
        let header = DatagramHeader::Packet {
            is_packet_pair: false,
            is_continuous_send: false,
            needs_data_arrival_rate: true,
            datagram_number: DatagramSequenceNumber::try_from(0x123456u32).unwrap()
        };
        let mut payload = Vec::new();

        // Act
        header.write(&mut payload).expect("Couldn't write header");

        // Assert
        assert_eq!(payload, vec![0b1000_0100u8, 0x56, 0x34, 0x12]);
    }  
}