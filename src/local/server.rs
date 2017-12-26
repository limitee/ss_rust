use define::ErrCode;
use define::ErrCode::*;

use std::thread;
use std::net::{TcpListener, TcpStream};

use local::protocol::Protocol;

pub struct LocalServer {
    ip: String,
    port: u32,
    listener: TcpListener,
    time_out: u64,
    remote_ip: String,
    remote_port: u32,
}

impl LocalServer {

    pub fn new(ip:&str, port:u32, remote_ip:&str, remote_port:u32, time_out:u64) -> Result<Self, ErrCode> {
        let url = format!("{}:{}", ip, port);
        let listener = TcpListener::bind(&url).or_else(|e|{
            error!("{}", e);
            Err(UrlErr)
        })?;
        Ok(LocalServer {
            ip: ip.to_string(),
            port: port,
            listener: listener,
            time_out: time_out,
            remote_ip: remote_ip.to_string(),
            remote_port: remote_port,
        })
    }

    //开启监听
    pub fn start(&mut self) {
        info!("local server start listening on {}:{}", self.ip, self.port);
        for stream_rst in self.listener.incoming() {
            let time_out = self.time_out;
            let remote_port = self.remote_port;
            if let Ok(stream) = stream_rst {
                let _ = Self::handle_stream(stream, &self.remote_ip, remote_port, time_out);
            }
        }
    }

    pub fn handle_stream(stream:TcpStream, remote_ip:&str, remote_port:u32, time_out:u64) -> Result<(), ErrCode> {
        let peer_addr = stream.peer_addr().or(Err(SocketErr))?;
        info!("{}", peer_addr);
        let ip = remote_ip.to_string();
        let _ = thread::spawn(move|| {
            let mut pro = Protocol::new(stream, ip, remote_port, time_out);
            let _ = pro.start();
        });
        Ok(())
    }
}
