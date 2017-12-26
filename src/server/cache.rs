use define::{ErrCode, Ip};
use define::ErrCode::*;

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use helper;

#[derive(Debug)]
struct Inner {
    map: RwLock<BTreeMap<String, Ip>>,
}

#[derive(Debug, Clone)]
pub struct DnsCache {
    inner: Arc<Inner>,
}

impl DnsCache {

    pub fn new() -> Self {
        let inner = Inner {map: RwLock::new(BTreeMap::new())};
        DnsCache {
            inner: Arc::new(inner),
        }
    }

    pub fn get_ip(&mut self, url:&str) -> Result<Ip, ErrCode> {
        let ip_op = {
            let map = self.inner.map.read().or(Err(LockErr))?;
            let ip_op = map.get(url);
            if let Some(ip) = ip_op {
                Some(ip.clone())
            } else {
                None
            }
        };
        if let Some(ip) = ip_op {
            return Ok(ip);
        } else {
            let ipv4_addr = helper::get_ip_addr(url)?;
            let parts = ipv4_addr.octets();
            let ip = Ip::new(parts[0], parts[1], parts[2], parts[3]);

            let mut map = self.inner.map.write().or(Err(LockErr))?;
            map.insert(url.to_string(), ip.clone());
            Ok(ip)
        }
    }
}
