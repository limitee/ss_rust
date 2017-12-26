use define::ErrCode;
use define::ErrCode::*;


use std::net::*;

use bytes::{BytesMut, BufMut};
use std::convert::AsRef;

extern crate dns_lookup;
use dns_lookup::lookup_host;

use std::ops::Not;

pub fn get_ip_addr(hostname:&str) -> Result<Ipv4Addr, ErrCode> {
    let ips: Vec<IpAddr> = lookup_host(hostname).or(Err(UrlErr))?;
    let address = ips.into_iter().next().ok_or(UrlErr)?;
    match address {
        IpAddr::V4(addr) => {
            Ok(addr)
        },
        _ => {
            Err(UrlErr)
        }
    }
}

pub fn encode<T: AsRef<[u8]>>(input:T) -> BytesMut {
    let data = input.as_ref();
    let mut buf = BytesMut::with_capacity(data.len());
    for set in data {
        buf.put_u8(set.not());
    }
    buf
}
