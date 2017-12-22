//! This is a binary running in the local environment
//!
//! You have to provide all needed configuration attributes via command line parameters,
//! or you could specify a configuration file. The format of configuration file is defined
//! in mod `config`.
//!

extern crate clap;
extern crate shadowsocks;
#[macro_use]
extern crate log;
extern crate time;

extern crate base_config;
use base_config::CFG;

extern crate base_log;
use base_log::init_base_log;

use shadowsocks::{Config, run_local, ErrCode, local};
use ErrCode::*;

/*
fn try_main() -> Result<(), ErrCode> {
    info!("{}", *CFG);
    info!("ShadowSocks {}", shadowsocks::VERSION);
    let json_obj = CFG.as_object().ok_or(JsonErr)?;
    let config = Config::parse_json_object(json_obj, true).or(Err(FileErr))?; 
    debug!("{}", config);
    run_local(config).unwrap();
    Ok(())
}
*/

fn try_main() -> Result<(), ErrCode> {
    info!("{}", *CFG);
    let local_addr = CFG["local_address"].as_str().ok_or(KeyFmtErr)?;
    let local_port = CFG["local_port"].as_u64().ok_or(KeyFmtErr)? as u32;
    let mut server = local::LocalServer::new(local_addr, local_port)?;
    let _ = server.start();
    Ok(())
}

fn main() {
    let _ = init_base_log();
    let _ = try_main();
}
