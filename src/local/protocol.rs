use define::ErrCode;
use define::ErrCode::*;

use std::net::{TcpStream};
use std::io::Read;
use std::io::Cursor;
use std::time::Duration;

extern crate byteorder;
use byteorder::{BigEndian, ReadBytesExt};

extern crate bytes;
use bytes::{BytesMut, BufMut};

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum ProStep {
    Start = 0, 
    Connect = 1,
}

impl ProStep {
    
    pub fn next(&mut self) {
        match *self {
            ProStep::Start => *self = ProStep::Connect,
            ProStep::Connect => panic!("not implement"),
        }
    }
}

pub struct Protocol {
    stream: TcpStream,
    buf: BytesMut,
    step: ProStep,
}

impl Protocol {
    
    pub fn new(stream:TcpStream, time_out:u64) -> Self {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(time_out)));
        Protocol {
            stream: stream,
            buf: BytesMut::with_capacity(1024),
            step: ProStep::Start,
        }
    }

    pub fn start(&mut self) -> Result<(), ErrCode> {
        let mut buf = vec![0u8; 1024];
        loop {
            let rst = self.stream.read(&mut buf);
            match rst {
                Ok(size) => {
                    info!("receive {} bytes data.", size);
                    self.buf.reserve(size);
                    self.buf.extend_from_slice(&buf[0..size]);
                    let _ = self.handle()?;
                },
                Err(e) => {
                    error!("{}", e);
                    break;
                }
            }
        }
        Ok(())
    }

    pub fn handle(&mut self) -> Result<(), ErrCode> {
        match self.step {
            ProStep::Start => {
                let head = self.get_head();
                self.step.next();
            },
            ProStep::Connect => {
            },
        }
        Ok(())
    }

    pub fn get_head(&mut self) -> Result<(), ErrCode> {
        info!("try to get head");
        if self.buf.len() < 3 {
            return Ok(());
        }
        let head_buf = self.buf.split_to(3);
        let mut cur = Cursor::new(head_buf);
        let version = cur.read_u16().or(Err(SocketErr))?;
        info!("the version is {:x}.", version);
        Ok(())
    }
}













