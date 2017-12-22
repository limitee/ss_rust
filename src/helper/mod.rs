use define::ErrCode;
use define::ErrCode::*;

extern crate trust_dns_resolver;

use std::net::*;
use trust_dns_resolver::Resolver;
use trust_dns_resolver::config::*;

pub fn get_ip_addr(domain:&str) -> Result<Ipv4Addr, ErrCode> {
    let resolver = Resolver::new(ResolverConfig::default(), ResolverOpts::default()).or(Err(UrlErr))?;
    let response = resolver.lookup_ip(domain).or(Err(UrlErr))?;
    let address = response.iter().next().ok_or(UrlErr)?;
    match address {
        IpAddr::V4(addr) => {
            Ok(addr)
        },
        _ => {
            Err(UrlErr)
        }
    }
}
