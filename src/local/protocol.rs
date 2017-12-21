use define::ErrCode;
use define::ErrCode::*;

use std::net::{TcpStream};
use std::io::{Read, Write};
use std::io::Cursor;
use std::time::Duration;
use std::collections::HashSet;

extern crate byteorder;
use byteorder::{BigEndian, ReadBytesExt};

extern crate bytes;
use bytes::{BytesMut, BufMut};

#[derive(Default)]
struct StartHead {
    version: u8,
    method: u8,
}

impl StartHead {
    
    pub fn set_version(&mut self, version:u8) {
        self.version = version;
    }

    pub fn set_method(&mut self, method:u8) {
        self.method = method;
    }
}

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
    start_head: StartHead,
}

impl Protocol {
    
    pub fn new(stream:TcpStream, time_out:u64) -> Self {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(time_out)));
        Protocol {
            stream: stream,
            buf: BytesMut::with_capacity(1024),
            step: ProStep::Start,
            start_head: Default::default(),
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
                let _ = self.get_start_head()?;
            },
            ProStep::Connect => {
                let _ = self.connect()?;
            },
        }
        Ok(())
    }

    pub fn connect(&mut self) -> Result<(), ErrCode> {
        info!("now is the connect step...");
        Ok(())
    }

    pub fn get_start_head(&mut self) -> Result<(), ErrCode> {
        //at least 3 bytes required
        if self.buf.len() < 3 {
            return Ok(());
        }
        let version;
        let method_len;
        {
            let mut cur = Cursor::new(&self.buf[0..2]);
            version = cur.read_u8().or(Err(SocketErr))?;
            method_len = cur.read_u8().or(Err(SocketErr))? as usize;
            //msg head is not finished
            if self.buf.len() < method_len + 2 {
                return Ok(());
            }
        }
        let _head_buf = self.buf.split_to(2);
        let mut method_list = HashSet::new();
        let method_buf = self.buf.split_to(method_len);
        let mut method_cur = Cursor::new(method_buf);
        for _ in 0..method_len {
            let method = method_cur.read_u8().or(Err(SocketErr))?;
            method_list.insert(method);
        }
        if version != 5 {
            return Err(UnImplementErr);
        }
        if !method_list.contains(&0_u8) {
            return Err(UnImplementErr);
        }
        self.start_head.set_version(version);
        self.start_head.set_method(0);

        let _ = self.back_start_head()?;
        self.step.next();
        Ok(())
    }

    pub fn back_start_head(&mut self) -> Result<(), ErrCode> {
        let back_head = vec![5, 0];
        let _ = self.stream.write_all(&back_head).or(Err(SocketErr))?;
        Ok(())
    }
}













