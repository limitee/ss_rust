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

#[derive(Default, Debug)]
struct Ip {
    first: u8,
    second: u8,
    third: u8,
    forth: u8,
}

impl Ip {
    
    pub fn new(fs:u8, s:u8, t:u8, f:u8) -> Self {
        Ip {
            first:fs,
            second:s,
            third:t,
            forth:f,
        }
    }
}

#[derive(Default, Debug)]
struct ConnectHead {
    version: u8,
    cmd: u8,
    rsv: u8,
    atyp: u8,
    ip: Ip,
    url: String,
    port: u16,
}

impl ConnectHead {

    pub fn set_version(&mut self, version:u8) {
        self.version = version;
    }

    pub fn set_cmd(&mut self, cmd:u8) {
        self.cmd = cmd;
    }

    pub fn set_rsv(&mut self, rsv:u8) {
        self.rsv = rsv;
    }

    pub fn set_atyp(&mut self, atyp:u8) {
        self.atyp = atyp;
    }

    pub fn set_port(&mut self, port:u16) {
        self.port = port;
    }
    
    pub fn set_ip(&mut self, ip:Ip) {
        self.ip = ip;
    }

    pub fn set_url(&mut self, url:&str) {
        self.url = url.to_string();
    }
}

#[derive(Default, Debug)]
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
        if self.buf.len() < 4 {
            return Ok(());
        }
        let version;
        let cmd;
        let rsv;
        let atyp;
        let mut head:ConnectHead = Default::default();
        {
            {
                let mut cur = Cursor::new(&self.buf[0..4]);
                version = cur.read_u8().or(Err(SocketErr))?;
                cmd = cur.read_u8().or(Err(SocketErr))?;
                rsv = cur.read_u8().or(Err(SocketErr))?;
                atyp = cur.read_u8().or(Err(SocketErr))?;

                head.set_version(version);
                head.set_cmd(cmd);
                head.set_rsv(rsv);
                head.set_atyp(atyp);
            }
            match atyp {
                1 => {
                    if self.buf.len() < 10 {
                        return Ok(());
                    }
                    let _ = self.buf.split_to(4);
                    let ip_buf = self.buf.split_to(4);
                    let mut ip_cur = Cursor::new(&ip_buf);
                    let fs = ip_cur.read_u8().or(Err(SocketErr))?;
                    let s = ip_cur.read_u8().or(Err(SocketErr))?;
                    let t = ip_cur.read_u8().or(Err(SocketErr))?;
                    let f = ip_cur.read_u8().or(Err(SocketErr))?;
                    let ip = Ip::new(fs, s, t, f);
                    head.set_ip(ip);
                },
                3 => {
                    if self.buf.len() < 5 {
                        return Ok(());
                    }
                    let domain_len = self.buf[4] as usize;
                    if self.buf.len() < 5 + domain_len + 2 {
                        return Ok(());
                    }
                    let _  = self.buf.split_to(5);
                    let buf = self.buf.split_to(domain_len);
                    let url = String::from_utf8(buf.to_vec()).or(Err(SocketErr))?;
                    head.set_url(&url);
                },
                _ => {
                    return Err(UnImplementErr);
                }
            }
        }
        let buf = self.buf.split_to(2);
        let mut cur = Cursor::new(&buf);
        let port = cur.read_u16::<BigEndian>().or(Err(SocketErr))?;
        head.set_port(port);
        info!("{:?}", head);
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













