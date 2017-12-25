use define::ErrCode;
use define::ErrCode::*;

use std::net::{TcpStream, Ipv4Addr};
use std::io::{Read, Write};
use std::io::Cursor;
use std::time::Duration;
use std::collections::HashSet;
use std::{thread};

extern crate byteorder;
use byteorder::{BigEndian, ReadBytesExt};

extern crate bytes;
use bytes::{BytesMut, BufMut};

use std::sync::mpsc::{channel};

use helper;

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

    pub fn to_ipv4(&self) -> Ipv4Addr {
        Ipv4Addr::new(self.first, self.second, self.third, self.forth)
    }
}

#[derive(Default, Debug)]
struct ConnectHead {
    atyp: u8,
    ip: Ip,
    url: String,
    port: u16,
}

impl ConnectHead {

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

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum ProStep {
    Connect = 1,
    ConnectTarget = 2,
}

impl ProStep {
    
    pub fn next(&mut self) {
        match *self {
            ProStep::Connect => *self = ProStep::ConnectTarget,
            ProStep::ConnectTarget => panic!("not implement"),
        }
    }
}

pub struct Protocol {
    stream: TcpStream, //stream to the client
    buf: BytesMut,
    step: ProStep,
    conn_head: ConnectHead,
    target_stream: Option<TcpStream>, //stream to the target
}

impl Protocol {
    
    pub fn new(stream:TcpStream, time_out:u64) -> Self {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(time_out)));
        Protocol {
            stream: stream,
            buf: BytesMut::with_capacity(1024),
            step: ProStep::Connect,
            conn_head: Default::default(),
            target_stream: None,
        }
    }

    pub fn start(&mut self) -> Result<(), ErrCode> {
        let mut buf = vec![0u8; 1024];
        loop {
            let rst = self.stream.read(&mut buf);
            match rst {
                Ok(size) => {
                    //info!("receive {} bytes data.", size);
                    if size == 0 {
                        break;
                    }
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
            ProStep::Connect => {
                let _ = self.connect()?;
                let rst = self.connect_target();
                match rst {
                    Ok(_) => {
                        //let _ = self.connect_success()?;
                        let _ = self.tunnel()?;
                    },
                    Err(_e) => {
                        let _ = self.connect_err()?;
                    },
                }
            },
            ProStep::ConnectTarget => {
            },
        }
        Ok(())
    }

    pub fn connect(&mut self) -> Result<(), ErrCode> {
        if self.buf.len() < 1 {
            return Ok(());
        }
        let atyp;
        let mut head:ConnectHead = Default::default();
        {
            {
                let mut cur = Cursor::new(&self.buf[0..1]);
                atyp = cur.read_u8().or(Err(SocketErr))?;
                head.set_atyp(atyp);
            }
            match atyp {
                1 => {
                    if self.buf.len() < 7 {
                        return Ok(());
                    }
                    let _ = self.buf.split_to(1);
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
                    if self.buf.len() < 2 {
                        return Ok(());
                    }
                    let domain_len = self.buf[1] as usize;
                    if self.buf.len() < 2 + domain_len + 2 {
                        return Ok(());
                    }
                    let _  = self.buf.split_to(2);
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
        self.conn_head = head;
        Ok(())
    }

    pub fn connect_target(&mut self) -> Result<(), ErrCode> {
        let atyp = self.conn_head.atyp;
        let ipv4_addr;
        if atyp == 1 {
            ipv4_addr = self.conn_head.ip.to_ipv4();
        } else {
            ipv4_addr = helper::get_ip_addr(&self.conn_head.url)?;
            //info!("resolver the site to ip:{}", ipv4_addr);
            //save the resolver ip to head
            let parts = ipv4_addr.octets();
            let ip = Ip::new(parts[0], parts[1], parts[2], parts[3]);
            self.conn_head.ip = ip;
        }
        info!("{}:{}:{} and buf len is {}.", self.conn_head.url, ipv4_addr, self.conn_head.port, self.buf.len());
        self.target_stream = Some(TcpStream::connect((ipv4_addr, self.conn_head.port)).or(Err(NetErr))?);
        Ok(())
    }

    pub fn tunnel(&mut self) -> Result<(), ErrCode> {
        let (sx, rx) = channel::<u64>();
        let stream = self.stream.try_clone().or(Err(SocketErr))?;
        let _ = stream.set_read_timeout(None).or(Err(SocketErr))?;

        let target_stream = self.target_stream.take().ok_or(SocketErr)?;

        let sx1 = sx.clone();
        let mut stream_read = stream.try_clone().or(Err(SocketErr))?;
        let mut target_stream_write = target_stream.try_clone().or(Err(SocketErr))?;
        let _th1 = thread::spawn(move || {
            let mut buf = vec![0u8; 1024];
            loop {
                let rst = stream_read.read(&mut buf);
                match rst {
                    Ok(size) => {
                        //info!("local stream receive {} bytes data.", size);
                        if size == 0 {
                            break;
                        }
                        let rst = target_stream_write.write_all(&buf[0..size]);
                        if rst.is_err() {
                            break;
                        }
                    },
                    Err(e) => {
                        error!("{}", e);
                        break;
                    }
                }
            }
            info!("local closed.");
            let _ = sx1.send(0);
        });

        let sx2 = sx.clone();
        let mut stream_write = stream.try_clone().or(Err(SocketErr))?;
        let mut target_stream_read = target_stream.try_clone().or(Err(SocketErr))?;
        let _th2 = thread::spawn(move || {
            let mut buf = vec![0u8; 1024];
            loop {
                let rst = target_stream_read.read(&mut buf);
                match rst {
                    Ok(size) => {
                        //info!("target stream receive {} bytes data.", size);
                        if size == 0 {
                            break;
                        }
                        let rst = stream_write.write_all(&buf[0..size]);
                        if rst.is_err() {
                            break;
                        }
                    },
                    Err(e) => {
                        error!("{}", e);
                        break;
                    }
                }
            }
            info!("target closed.");
            let _ = sx2.send(0);
        });

        //th1 or th2 finished, will return.
        let _ = rx.recv().or(Err(SocketErr))?;

        Err(SocketErr)
    }

    ///connect the target success
    pub fn connect_success(&mut self) -> Result<(), ErrCode> {
        Ok(())
    }
    
    ///no used yet
    pub fn connect_err(&mut self) -> Result<(), ErrCode> {
        Ok(())
    }

}








