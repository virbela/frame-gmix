use std::io;

use byteorder::{BigEndian, ByteOrder};
use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use super::message::{RequestMessage, ResponseMessage};

pub struct Client;

impl Decoder for Client {
    type Item = RequestMessage;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let size = {
            if src.len() < 2 {
                return Ok(None);
            }
            BigEndian::read_u16(src.as_ref()) as usize
        };

        if src.len() >= size + 2 {
            let _ = src.split_to(2);
            let buf = src.split_to(size);
            Ok(Some(serde_json::from_slice::<RequestMessage>(&buf)?))
        } else {
            Ok(None)
        }
    }
}

impl Encoder<ResponseMessage> for Client {
    type Error = io::Error;
    fn encode(&mut self, msg: ResponseMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let msg = serde_json::to_string(&msg).unwrap();
        let msg_ref: &[u8] = msg.as_ref();

        dst.reserve(msg_ref.len() + 2);
        dst.put_u16(msg_ref.len() as u16);
        dst.put(msg_ref);

        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Server {
    size: Option<usize>,
}

impl Default for Server {
    fn default() -> Self {
        Self { size: None }
    }
}

impl Decoder for Server {
    type Item = RequestMessage;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        println!("incoming buff: {:?}", &src);
        println!("self size: {:?}", &self.size);
        loop {
            match self.size {
                Some(size) => {
                    return if size <= src.len() {
                        let msg = serde_json::from_slice(&src[..size])?;
                        src.advance(size);
                        self.size = None;
                        println!("incoming buff: {:?}", &src);
                        Ok(Some(msg))
                    } else {
                        println!("none");
                        Ok(None)
                    }
                }
                None => {
                    if src.len() < 4 {
                        println!("None");
                        return Ok(None);
                    }

                    self.size = Some(BigEndian::read_u32(&src[..4]) as usize);

                    src.advance(4);
                }
            }
        }
    }
}

impl Encoder<ResponseMessage> for Server {
    type Error = io::Error;

    fn encode(&mut self, msg: ResponseMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let base = dst.len();

        dst.extend_from_slice(&[0; 4]);

        serde_json::to_writer(dst.writer(), &msg).unwrap();

        let len = (dst.len() - base) as u32;

        // TODO: no BigEndian?
        (&mut dst[base..]).put_u32(len);

        Ok(())
    }
}
