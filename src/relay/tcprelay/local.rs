//! Relay for TCP server that running on local environment

use super::socks5_local;
use define::RunRst;

/// Starts a TCP local server
pub fn run() -> RunRst {
    socks5_local::run()
}
