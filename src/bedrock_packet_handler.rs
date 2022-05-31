use std::{io::Read, net::SocketAddr};
use log::{debug, error};
use flate2::read::DeflateDecoder;

use crate::{bedrock_reader::BedrockReader, error::Result, utils::to_hex};

pub struct BedrockPacketHandler {
}

impl BedrockPacketHandler {
    pub fn new() -> Self {
        BedrockPacketHandler {            
        }
    }

    pub fn handle_raknet_packet(&mut self, _addr: SocketAddr, _guid: u64, payload: &[u8]) {
        if payload.len() == 0 {
            error!("Emtpy packet payload");
            return;
        }
        if payload[0] == 0xfe {
            let payload = &payload[1..];
            let mut decoder = DeflateDecoder::new(payload);
            let mut buf = Vec::new();
            match decoder.read_to_end(&mut buf) {
                Ok(_) => {
                    debug!("Decompressed data, size {}: {}", buf.len(), to_hex(&buf, 40));
                    let mut reader = BedrockReader::new(&buf);
                    while reader.has_more() {
                        if let Err(err) = self.read_and_handle_bedrock_packet(&mut reader) {
                            error!("Error handling bedrock packet: {:?}", err);
                            return;
                        }
                    }
                },
                Err(err) => error!("Error decompressing data: {:?}", err),
            };
        } else {
            error!("Invalid message ID: {}", payload[0]);
        }
    }

    fn read_and_handle_bedrock_packet(&mut self, reader: &mut BedrockReader) -> Result<()> {
        let length = reader.read_varint_to_u32()?;
        let payload = reader.read_bytes_as_slice(length as usize)?;
        self.handle_bedrock_packet(payload)
    }

    fn handle_bedrock_packet(&mut self, payload: &[u8]) -> Result<()> {
        let mut reader = BedrockReader::new(payload);
        let id = reader.read_varint_to_u32()?;       
        debug!("Bedrock packet with ID: {} and length (including ID): {}", id, payload.len());
        match id {
            0x01 => self.handle_login_packet(&mut reader)?,
            _ => error!("Unknown ID: {}", id),
        }
        Ok(())
    }

    fn handle_login_packet(&mut self, reader: &mut BedrockReader) -> Result<()> {
        let protocol_version = reader.read_u32_be()?;
        let payload_length = reader.read_varint_to_u32()?;
        debug!("Login packet. Protocol version: {}. Payload length: {}", protocol_version, payload_length);

        let cert_chain_length = reader.read_u32_le()?;
        let cert_chain = std::str::from_utf8(reader.read_bytes_as_slice(cert_chain_length as usize)?)?;
        debug!("Cert chain length: {}. Cert chain:\r\n{}", cert_chain_length, cert_chain);
        // TODO: Parse and verify certificate chain
        
        let skin_data_length = reader.read_u32_le()?;
        let skin_data = std::str::from_utf8(reader.read_bytes_as_slice(skin_data_length as usize)?)?;
        debug!("Skin length: {}. Skin data:\r\n{}", skin_data_length, skin_data);        
        // TODO: Parse and handle skin data
        Ok(())
    }

}