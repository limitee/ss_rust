use define::ErrCode;
use define::ErrCode::*;

use std::thread;
use std::net::{TcpListener, TcpStream};

mod protocol;
use self::protocol::Protocol;

mod cache;
use self::cache::DnsCache;

pub struct Server {
    ip: String,
    port: u32,
    listener: TcpListener,
    time_out: u64,
    cache: DnsCache,
}

impl Server {

    pub fn new(ip:&str, port:u32, time_out:u64) -> Result<Self, ErrCode> {
        let url = format!("{}:{}", ip, port);
        let listener = TcpListener::bind(&url).or_else(|e|{
            error!("{}", e);
            Err(UrlErr)
        })?;
        let cache = DnsCache::new();
        Ok(Server {
            ip: ip.to_string(),
            port: port,
            listener: listener,
            time_out: time_out,
            cache: cache,
        })
    }

    //开启监听
    pub fn start(&mut self) {
        info!("local server start listening on {}:{}", self.ip, self.port);
        for stream_rst in self.listener.incoming() {
            let time_out = self.time_out;
            let cache = self.cache.clone();
            if let Ok(stream) = stream_rst {
                let _ = Self::handle_stream(stream, time_out, cache);
            }
        }
    }

    pub fn handle_stream(stream:TcpStream, time_out:u64, cache:DnsCache) -> Result<(), ErrCode> {
        let peer_addr = stream.peer_addr().or(Err(SocketErr))?;
        info!("{}", peer_addr);
        let _ = thread::spawn(move|| {
            let mut pro = Protocol::new(stream, time_out, cache);
            let _ = pro.start();
        });
        Ok(())
    }
}
