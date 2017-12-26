#[macro_use]
extern crate log;
extern crate serde_json;

pub mod local;
pub mod server;

pub mod define;
pub use define::*;

mod helper;

extern crate bytes;
extern crate byteorder;
use byteorder::ReadBytesExt;

extern crate trust_dns_resolver;

