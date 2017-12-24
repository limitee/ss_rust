use define::ErrCode;
use define::ErrCode::*;

use std::thread;
use std::net::{TcpListener, TcpStream};

use local::protocol::Protocol;

pub struct LocalServer {
    ip: String,
    port: u32,
    listener: TcpListener,
}

impl LocalServer {

    pub fn new(ip:&str, port:u32) -> Result<Self, ErrCode> {
        let url = format!("{}:{}", ip, port);
        let listener = TcpListener::bind(&url).or_else(|e|{
            error!("{}", e);
            Err(UrlErr)
        })?;
        Ok(LocalServer {
            ip: ip.to_string(),
            port: port,
            listener: listener,
        })
    }

    //开启监听
    pub fn start(&mut self) {
        info!("local server start listening on {}:{}", self.ip, self.port);
        for stream_rst in self.listener.incoming() {
            if let Ok(stream) = stream_rst {
                let _ = Self::handle_stream(stream);
            }
        }
    }

    pub fn handle_stream(stream:TcpStream) -> Result<(), ErrCode> {
        let peer_addr = stream.peer_addr().or(Err(SocketErr))?;
        info!("{}", peer_addr);
        let _ = thread::spawn(move|| {
            let mut pro = Protocol::new(stream, 20000);
            let _ = pro.start();
        });
        Ok(())
    }
}
