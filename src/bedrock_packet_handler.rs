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
        debug!("Bedrock packet with id: {} and length including id: {}", id, payload.len());
        Ok(())
    }
}