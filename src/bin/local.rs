extern crate ss_rust;
#[macro_use]
extern crate log;

extern crate base_config;
use base_config::CFG;

extern crate base_log;
use base_log::init_base_log;

use ss_rust::{ErrCode, local};
use ErrCode::*;

fn try_main() -> Result<(), ErrCode> {
    info!("{}", *CFG);
    let local_addr = CFG["local_address"].as_str().ok_or(KeyFmtErr)?;
    let local_port = CFG["local_port"].as_u64().ok_or(KeyFmtErr)? as u32;

    let server = CFG["server"].as_str().ok_or(KeyFmtErr)?;
    let server_port = CFG["server_port"].as_u64().ok_or(KeyFmtErr)? as u32;

    let time_out = CFG["timeout"].as_u64().ok_or(KeyFmtErr)?;
    let mut server = local::LocalServer::new(local_addr, local_port, server, server_port, time_out)?;
    let _ = server.start();
    Ok(())
}

fn main() {
    let _ = init_base_log();
    let _ = try_main();
}
