//! Inspector shim role.
//!
//! Connects to the MCP server's local socket and relays bytes in both directions between
//! the harness's stdio and that socket.
//!
//! From the harness's perspective we're an ordinary `kittest_inspector` (msgpack framed
//! messages on stdin/stdout). The MCP server sees the same framed bytes on the other end of
//! the socket. We don't parse or interpret anything here — pure byte relay keeps the shim
//! independent of protocol revisions.

use std::io::{Read as _, Write as _};
use std::thread;

use egui_inspection::transport::socket_name;
use interprocess::local_socket::{Stream, prelude::*};

pub fn run(socket: &str) -> anyhow::Result<()> {
    let name = socket_name(socket).map_err(|e| anyhow::anyhow!("socket name {socket}: {e}"))?;
    let stream = Stream::connect(name).map_err(|e| anyhow::anyhow!("connect {socket}: {e}"))?;
    let (mut reader, mut stdin_to_socket) = stream.split();

    // Thread A: stdin (from harness) → socket (to MCP server).
    let t_in = thread::Builder::new()
        .name("kittest-mcp-shim-stdin".into())
        .spawn(move || {
            let mut stdin = std::io::stdin().lock();
            let _ = std::io::copy(&mut stdin, &mut stdin_to_socket);
            // EOF on stdin or write error → drop the send half so the peer sees EOF on the
            // write direction.
            drop(stdin_to_socket);
        })?;

    // Thread B: socket (from MCP server) → stdout (to harness).
    // Runs on main thread so the process exits when stdout closes.
    let mut stdout = std::io::stdout().lock();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        match reader.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                if stdout.write_all(&buf[..n]).is_err() {
                    break;
                }
                let _ = stdout.flush();
            }
        }
    }

    let _ = t_in.join();
    Ok(())
}
