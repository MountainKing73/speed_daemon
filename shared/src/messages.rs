use bytes::{Buf, BufMut, BytesMut};
use log::debug;
use std::str;
use tokio_util::codec::{Decoder, Encoder};

use crate::network::{Camera, Ticket};

#[derive(Debug, PartialEq, Eq)]
pub enum MessageType {
    Error(String),
    Plate(String, u32),
    Ticket(Ticket),
    WantHeartbeat(u32),
    HeartBeat,
    IAmCamera(Camera),
    IAmDispatcher(Vec<u16>),
}

fn get_string(src: &mut BytesMut) -> Result<Option<String>, std::io::Error> {
    let str_end = src[0] as usize + 1;
    if src.len() < str_end {
        return Ok(None);
    }
    let sub_bytes = &src[1..str_end];
    let s = match str::from_utf8(sub_bytes) {
        Ok(s) => s,
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
    };
    let result = s.to_string();
    src.advance(str_end);

    Ok(Some(result))
}

fn get_u32(src: &mut BytesMut) -> Result<Option<u32>, std::io::Error> {
    if src.len() < 4 {
        return Ok(None);
    }

    let num_bytes: [u8; 4] = match src[0..4].try_into() {
        Ok(n) => n,
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
    };
    src.advance(4);
    Ok(Some(u32::from_be_bytes(num_bytes)))
}

fn get_u16(src: &mut BytesMut) -> Result<Option<u16>, std::io::Error> {
    if src.len() < 2 {
        return Ok(None);
    }

    let num_bytes: [u8; 2] = match src[0..2].try_into() {
        Ok(n) => n,
        Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
    };
    src.advance(2);
    Ok(Some(u16::from_be_bytes(num_bytes)))
}

pub struct MessageDecoder {}

impl Decoder for MessageDecoder {
    type Item = MessageType;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        debug!("msg: {:?}", src);
        // Get the message type and advance the buffer past the indicator
        let msg_type = src[0];
        src.advance(1);
        match msg_type {
            0x10 => {
                let string = get_string(src);
                match string {
                    Ok(result) => match result {
                        Some(s) => Ok(Some(MessageType::Error(s))),
                        None => Ok(None),
                    },
                    Err(e) => Err(e),
                }
            }
            0x20 => {
                let string = get_string(src);
                let plate_string = match string {
                    Ok(result) => match result {
                        Some(s) => s,
                        None => return Ok(None),
                    },
                    Err(e) => return Err(e),
                };

                let num = get_u32(src);
                let timestamp = match num {
                    Ok(result) => match result {
                        Some(n) => n,
                        None => return Ok(None),
                    },
                    Err(e) => return Err(e),
                };

                Ok(Some(MessageType::Plate(plate_string, timestamp)))
            }
            0x21 => {
                let string = get_string(src);
                let plate_string = match string {
                    Ok(result) => match result {
                        Some(s) => s,
                        None => return Ok(None),
                    },
                    Err(e) => return Err(e),
                };

                let num = get_u16(src);
                let road = match num {
                    Ok(result) => match result {
                        Some(n) => n,
                        None => return Ok(None),
                    },
                    Err(e) => return Err(e),
                };

                let num = get_u16(src);
                let mile1 = match num {
                    Ok(result) => match result {
                        Some(n) => n,
                        None => return Ok(None),
                    },
                    Err(e) => return Err(e),
                };
                let num = get_u32(src);
                let timestamp1 = match num {
                    Ok(result) => match result {
                        Some(n) => n,
                        None => return Ok(None),
                    },
                    Err(e) => return Err(e),
                };
                let num = get_u16(src);
                let mile2 = match num {
                    Ok(result) => match result {
                        Some(n) => n,
                        None => return Ok(None),
                    },
                    Err(e) => return Err(e),
                };
                let num = get_u32(src);
                let timestamp2 = match num {
                    Ok(result) => match result {
                        Some(n) => n,
                        None => return Ok(None),
                    },
                    Err(e) => return Err(e),
                };
                let num = get_u16(src);
                let speed = match num {
                    Ok(result) => match result {
                        Some(n) => n,
                        None => return Ok(None),
                    },
                    Err(e) => return Err(e),
                };
                Ok(Some(MessageType::Ticket(Ticket {
                    plate: plate_string,
                    road,
                    mile1,
                    timestamp1,
                    mile2,
                    timestamp2,
                    speed,
                })))
            }
            0x40 => {
                let num = get_u32(src);
                let interval = match num {
                    Ok(result) => match result {
                        Some(n) => n,
                        None => return Ok(None),
                    },
                    Err(e) => return Err(e),
                };

                Ok(Some(MessageType::WantHeartbeat(interval)))
            }
            0x41 => Ok(Some(MessageType::HeartBeat)),
            0x80 => {
                let num = get_u16(src);
                let road = match num {
                    Ok(result) => match result {
                        Some(n) => n,
                        None => return Ok(None),
                    },
                    Err(e) => return Err(e),
                };

                let num = get_u16(src);
                let mile = match num {
                    Ok(result) => match result {
                        Some(n) => n,
                        None => return Ok(None),
                    },
                    Err(e) => return Err(e),
                };

                let num = get_u16(src);
                let limit = match num {
                    Ok(result) => match result {
                        Some(n) => n,
                        None => return Ok(None),
                    },
                    Err(e) => return Err(e),
                };

                Ok(Some(MessageType::IAmCamera(Camera { road, mile, limit })))
            }
            0x81 => {
                let num_roads = src[0];
                src.advance(1);

                let mut roads: Vec<u16> = vec![];

                for _ in 0..num_roads {
                    let num = get_u16(src);
                    let road = match num {
                        Ok(result) => match result {
                            Some(n) => n,
                            None => return Ok(None),
                        },
                        Err(e) => return Err(e),
                    };

                    roads.push(road);
                }

                Ok(Some(MessageType::IAmDispatcher(roads)))
            }
            m => unimplemented!("Message Type not implemented: {}", m),
        }
    }
}

pub struct MessageEncoder {}

impl Encoder<MessageType> for MessageEncoder {
    type Error = std::io::Error;

    fn encode(&mut self, item: MessageType, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            MessageType::Error(s) => {
                dst.reserve(2 + s.len());

                // insert message type and length of string before adding string
                dst.put_u8(0x10);
                dst.put_u8(s.len() as u8);
                dst.extend_from_slice(s.as_bytes());
            }
            MessageType::Plate(plate, timestamp) => {
                // 1 byte for the type and 4 for the timestamp
                dst.reserve(6 + plate.len());

                dst.put_u8(0x20);
                dst.put_u8(plate.len() as u8);
                dst.extend_from_slice(plate.as_bytes());
                dst.put_u32(timestamp);
            }
            MessageType::Ticket(t) => {
                dst.reserve(17 + t.plate.len());

                dst.put_u8(0x21);
                dst.put_u8(t.plate.len() as u8);
                dst.extend_from_slice(t.plate.as_bytes());
                dst.put_u16(t.road);
                dst.put_u16(t.mile1);
                dst.put_u32(t.timestamp1);
                dst.put_u16(t.mile2);
                dst.put_u32(t.timestamp2);
                dst.put_u16(t.speed);
            }
            MessageType::WantHeartbeat(i) => {
                dst.reserve(5);

                dst.put_u8(0x40);
                dst.put_u32(i);
            }
            MessageType::HeartBeat => dst.put_u8(0x41),
            MessageType::IAmCamera(c) => {
                dst.reserve(7);

                dst.put_u8(0x80);
                dst.put_u16(c.road);
                dst.put_u16(c.mile);
                dst.put_u16(c.limit);
            }
            MessageType::IAmDispatcher(r) => {
                dst.reserve(2 + r.len());

                dst.put_u8(0x81);
                dst.put_u8(r.len() as u8);
                for road in r {
                    dst.put_u16(road);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::sink::SinkExt;
    use tokio_stream::StreamExt;
    use tokio_util::codec::{FramedRead, FramedWrite};

    #[tokio::test]
    async fn test_decode_error() {
        let decoder = MessageDecoder {};
        let msg: [u8; 5] = [0x10, 0x03, 0x62, 0x61, 0x64];

        let mut reader = FramedRead::new(&msg[..], decoder);

        let frame = reader.next().await.unwrap().unwrap();
        assert_eq!(frame, MessageType::Error(String::from("bad")));
    }

    #[tokio::test]
    async fn test_decode_plate() {
        let decoder = MessageDecoder {};
        let msg: [u8; 13] = [
            0x20, 0x07, 0x52, 0x45, 0x30, 0x35, 0x42, 0x4b, 0x47, 0x00, 0x01, 0xe2, 0x40,
        ];

        let mut reader = FramedRead::new(&msg[..], decoder);

        let frame = reader.next().await.unwrap().unwrap();

        assert_eq!(frame, MessageType::Plate(String::from("RE05BKG"), 123456,));
    }

    #[tokio::test]
    async fn test_decode_ticket() {
        let decoder = MessageDecoder {};
        let msg: [u8; 25] = [
            0x21, 0x07, 0x52, 0x45, 0x30, 0x35, 0x42, 0x4b, 0x47, 0x01, 0x70, 0x04, 0xd2, 0x00,
            0x0f, 0x42, 0x40, 0x04, 0xd3, 0x00, 0x0f, 0x42, 0x7c, 0x17, 0x70,
        ];

        let mut reader = FramedRead::new(&msg[..], decoder);

        let frame = reader.next().await.unwrap().unwrap();

        assert_eq!(
            frame,
            MessageType::Ticket(Ticket {
                plate: String::from("RE05BKG"),
                road: 368,
                mile1: 1234,
                timestamp1: 1000000,
                mile2: 1235,
                timestamp2: 1000060,
                speed: 6000,
            })
        );
    }

    #[tokio::test]
    async fn test_decode_want_heartbeat() {
        let decoder = MessageDecoder {};
        let msg: [u8; 5] = [0x40, 0x00, 0x00, 0x04, 0xdb];

        let mut reader = FramedRead::new(&msg[..], decoder);

        let frame = reader.next().await.unwrap().unwrap();

        assert_eq!(frame, MessageType::WantHeartbeat(1243));
    }

    #[tokio::test]
    async fn test_decode_heartbeat() {
        let decoder = MessageDecoder {};
        let msg: [u8; 1] = [0x41];

        let mut reader = FramedRead::new(&msg[..], decoder);

        let frame = reader.next().await.unwrap().unwrap();

        assert_eq!(frame, MessageType::HeartBeat);
    }

    #[tokio::test]
    async fn test_decode_iamcamera() {
        let decoder = MessageDecoder {};
        let msg: [u8; 7] = [0x80, 0x00, 0x42, 0x00, 0x64, 0x00, 0x3c];

        let mut reader = FramedRead::new(&msg[..], decoder);

        let frame = reader.next().await.unwrap().unwrap();

        assert_eq!(
            frame,
            MessageType::IAmCamera(Camera {
                road: 66,
                mile: 100,
                limit: 60
            })
        );
    }

    #[tokio::test]
    async fn test_decode_iamdispatcher() {
        let decoder = MessageDecoder {};
        let msg: [u8; 8] = [0x81, 0x03, 0x00, 0x42, 0x01, 0x70, 0x13, 0x88];

        let mut reader = FramedRead::new(&msg[..], decoder);

        let frame = reader.next().await.unwrap().unwrap();

        assert_eq!(frame, MessageType::IAmDispatcher(vec![66, 368, 5000]));
    }

    #[tokio::test]
    async fn test_encode_error() {
        let buffer = Vec::new();
        let encoder = MessageEncoder {};
        let error = String::from("illegal msg");

        let mut writer = FramedWrite::new(buffer, encoder);

        writer.send(MessageType::Error(error)).await.unwrap();

        let buffer = writer.get_ref();

        assert_eq!(
            buffer.as_slice(),
            [
                0x10, 0x0b, 0x69, 0x6c, 0x6c, 0x65, 0x67, 0x61, 0x6c, 0x20, 0x6d, 0x73, 0x67
            ]
        );
    }

    #[tokio::test]
    async fn test_encode_plate() {
        let buffer = Vec::new();
        let encoder = MessageEncoder {};

        let mut writer = FramedWrite::new(buffer, encoder);

        writer
            .send(MessageType::Plate(String::from("UN1X"), 1000))
            .await
            .unwrap();

        let buffer = writer.get_ref();

        assert_eq!(
            buffer.as_slice(),
            [0x20, 0x04, 0x55, 0x4e, 0x31, 0x58, 0x00, 0x00, 0x03, 0xe8]
        );
    }

    #[tokio::test]
    async fn test_encode_ticket() {
        let buffer = Vec::new();
        let encoder = MessageEncoder {};
        let ticket = Ticket {
            plate: String::from("UN1X"),
            road: 66,
            mile1: 100,
            timestamp1: 123456,
            mile2: 110,
            timestamp2: 123816,
            speed: 10000,
        };

        let mut writer = FramedWrite::new(buffer, encoder);

        writer.send(MessageType::Ticket(ticket)).await.unwrap();

        let buffer = writer.get_ref();

        assert_eq!(
            buffer.as_slice(),
            [
                0x21, 0x04, 0x55, 0x4e, 0x31, 0x58, 0x00, 0x42, 0x00, 0x64, 0x00, 0x01, 0xe2, 0x40,
                0x00, 0x6e, 0x00, 0x01, 0xe3, 0xa8, 0x27, 0x10
            ]
        );
    }

    #[tokio::test]
    async fn test_encode_want_heartbeat() {
        let buffer = Vec::new();
        let encoder = MessageEncoder {};

        let mut writer = FramedWrite::new(buffer, encoder);

        writer.send(MessageType::WantHeartbeat(10)).await.unwrap();

        let buffer = writer.get_ref();

        assert_eq!(buffer.as_slice(), [0x40, 0x00, 0x00, 0x00, 0x0a]);
    }

    #[tokio::test]
    async fn test_encode_heartbeat() {
        let buffer = Vec::new();
        let encoder = MessageEncoder {};

        let mut writer = FramedWrite::new(buffer, encoder);

        writer.send(MessageType::HeartBeat).await.unwrap();

        let buffer = writer.get_ref();

        assert_eq!(buffer.as_slice(), [0x41]);
    }

    #[tokio::test]
    async fn test_encode_iamcamera() {
        let buffer = Vec::new();
        let encoder = MessageEncoder {};

        let mut writer = FramedWrite::new(buffer, encoder);

        writer
            .send(MessageType::IAmCamera(Camera {
                road: 368,
                mile: 1234,
                limit: 40,
            }))
            .await
            .unwrap();

        let buffer = writer.get_ref();

        assert_eq!(
            buffer.as_slice(),
            [0x80, 0x01, 0x70, 0x04, 0xd2, 0x00, 0x28]
        );
    }

    #[tokio::test]
    async fn test_encode_iamdispatcher() {
        let buffer = Vec::new();
        let encoder = MessageEncoder {};

        let mut writer = FramedWrite::new(buffer, encoder);

        writer
            .send(MessageType::IAmDispatcher(vec![66, 368, 5000]))
            .await
            .unwrap();

        let buffer = writer.get_ref();

        assert_eq!(
            buffer.as_slice(),
            [0x81, 0x03, 0x00, 0x42, 0x01, 0x70, 0x13, 0x88]
        );
    }
}
