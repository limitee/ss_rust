use define::{Ip, ErrCode};
use define::ErrCode::*;

use std::net::{Shutdown, TcpStream, Ipv4Addr};
use std::net::ToSocketAddrs;
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
use helper::encode;

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
    ConnectTarget = 2,
}

impl ProStep {
    
    pub fn next(&mut self) {
        match *self {
            ProStep::Start => *self = ProStep::Connect,
            ProStep::Connect => *self = ProStep::ConnectTarget,
            ProStep::ConnectTarget => panic!("not implement"),
        }
    }
}

pub struct Protocol {
    stream: TcpStream, //stream to the client
    buf: BytesMut,
    step: ProStep,
    start_head: StartHead,
    conn_head: ConnectHead,
    target_stream: Option<TcpStream>, //stream to the target
    remote_ip: String,
    remote_port: u32,
    time_out: u64,
}

impl Protocol {
    
    pub fn new(stream:TcpStream, remote_ip:String, remote_port:u32, time_out:u64) -> Self {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(time_out)));
        Protocol {
            stream: stream,
            buf: BytesMut::with_capacity(1024),
            step: ProStep::Start,
            start_head: Default::default(),
            conn_head: Default::default(),
            target_stream: None,
            time_out: time_out,
            remote_ip: remote_ip,
            remote_port: remote_port,
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
            ProStep::Start => {
                let _ = self.get_start_head()?;
            },
            ProStep::Connect => {
                let _ = self.connect()?;
                let rst = self.connect_target();
                match rst {
                    Ok(_) => {
                        let _ = self.connect_success()?;
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
        info!("{:?} and buf len is {}.", head, self.buf.len());
        self.conn_head = head;
        self.step.next();
        Ok(())
    }

    pub fn connect_target(&mut self) -> Result<(), ErrCode> {
        /*
        let atyp = self.conn_head.atyp;
        let ipv4_addr;
        if atyp == 1 {
            ipv4_addr = self.conn_head.ip.to_ipv4();
        } else {
            ipv4_addr = helper::get_ip_addr(&self.conn_head.url)?;
            info!("resolver the site to ip:{}", ipv4_addr);
            //save the resolver ip to head
            let parts = ipv4_addr.octets();
            let ip = Ip::new(parts[0], parts[1], parts[2], parts[3]);
            self.conn_head.ip = ip;
        }
        self.target_stream = Some(TcpStream::connect((ipv4_addr, self.conn_head.port)).or(Err(NetErr))?);
        */
        let time_out = Duration::from_secs(self.time_out);
        let uri = format!("{}:{}", self.remote_ip, self.remote_port);
        let addr = uri.parse().or(Err(NetErr))?;

        let target_stream = TcpStream::connect_timeout(&addr, time_out).or(Err(NetErr))?;
        self.target_stream = Some(target_stream);

        let _ = self.write_ss_head()?;
        Ok(())
    }

    ///send the ss head
    pub fn write_ss_head(&mut self) -> Result<(), ErrCode> {
        let mut buf = BytesMut::new();
        let atyp = self.conn_head.atyp;
        buf.reserve(1);
        buf.put_u8(atyp);
        if atyp == 1 {
            buf.reserve(4);
            let ip = &self.conn_head.ip;
            buf.put_u8(ip.first);
            buf.put_u8(ip.second);
            buf.put_u8(ip.third);
            buf.put_u8(ip.forth);
        } else {
            let url_bytes = self.conn_head.url.as_bytes();
            buf.reserve(url_bytes.len() + 1);
            buf.put_u8(url_bytes.len() as u8);
            buf.put_slice(&url_bytes);
        }
        buf.reserve(2);
        buf.put_u16::<BigEndian>(self.conn_head.port);
        let mut stream = self.target_stream.as_ref().ok_or(NetErr)?;
        let _ = stream.write_all(&encode(&buf)).or(Err(SocketErr))?;
        //write the upload buf
        let _ = stream.write_all(&encode(&self.buf)).or(Err(SocketErr))?;
        Ok(())
    }

    pub fn tunnel(&mut self) -> Result<(), ErrCode> {
        let (sx, rx) = channel::<u64>();
        let stream = self.stream.try_clone().or(Err(SocketErr))?;
        let target_stream = self.target_stream.take().ok_or(SocketErr)?;

        //write time out 1 minute
        let _ = stream.set_write_timeout(Some(Duration::from_millis(60*1000))).or(Err(SocketErr))?;
        let _ = target_stream.set_write_timeout(Some(Duration::from_millis(60*1000))).or(Err(SocketErr))?;

        //read time out 6 minute 
        //let _ = stream.set_read_timeout(Some(Duration::from_millis(6*60*1000))).or(Err(SocketErr))?;
        //let _ = target_stream.set_read_timeout(Some(Duration::from_millis(6*60*1000))).or(Err(SocketErr))?;

        let _ = stream.set_read_timeout(None).or(Err(SocketErr))?;
        let _ = target_stream.set_read_timeout(None).or(Err(SocketErr))?;

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
                        let rst = target_stream_write.write_all(&encode(&buf[0..size]));
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
                        let rst = stream_write.write_all(&encode(&buf[0..size]));
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
            let _ = sx2.send(0);
        });

        //th1 or th2 finished, will return.
        let _ = rx.recv().or(Err(SocketErr))?;
        let _ = stream.shutdown(Shutdown::Both);
        let _ = target_stream.shutdown(Shutdown::Both);

        Err(SocketErr)
    }

    ///connect the target success
    pub fn connect_success(&mut self) -> Result<(), ErrCode> {
        let mut buf = BytesMut::from(vec![5, 0, 0]);
        let atyp = self.conn_head.atyp;
        buf.reserve(1);
        buf.put_u8(atyp);
        if atyp == 1 {
            buf.reserve(4);
            let ip = &self.conn_head.ip;
            buf.put_u8(ip.first);
            buf.put_u8(ip.second);
            buf.put_u8(ip.third);
            buf.put_u8(ip.forth);
        } else {
            let url_bytes = self.conn_head.url.as_bytes();
            buf.reserve(url_bytes.len() + 1);
            buf.put_u8(url_bytes.len() as u8);
            buf.put_slice(&url_bytes);
        }
        buf.reserve(2);
        buf.put_u16::<BigEndian>(self.conn_head.port);
        let _ = self.stream.write_all(&buf).or(Err(SocketErr))?;
        Ok(())
    }
    
    pub fn connect_err(&mut self) -> Result<(), ErrCode> {
        let mut buf = BytesMut::from(vec![5, 1, 0]);
        let atyp = self.conn_head.atyp;
        buf.reserve(1);
        buf.put_u8(atyp);
        if atyp == 1 {
            buf.reserve(4);
            let ip = &self.conn_head.ip;
            buf.put_u8(ip.first);
            buf.put_u8(ip.second);
            buf.put_u8(ip.third);
            buf.put_u8(ip.forth);
        } else {
            let url_bytes = self.conn_head.url.as_bytes();
            buf.reserve(url_bytes.len() + 1);
            buf.put_u8(url_bytes.len() as u8);
            buf.put_slice(&url_bytes);
        }
        buf.reserve(2);
        buf.put_u16::<BigEndian>(self.conn_head.port);
        let _ = self.stream.write_all(&buf).or(Err(SocketErr))?;
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













