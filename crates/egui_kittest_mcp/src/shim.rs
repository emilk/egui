//! Inspector shim role.
//!
//! Connects to the MCP server's unix domain socket and relays bytes in both directions
//! between the harness's stdio and that socket.
//!
//! From the harness's perspective we're an ordinary `kittest_inspector` (msgpack framed
//! messages on stdin/stdout). The MCP server sees the same framed bytes on the other end of
//! the socket. We don't parse or interpret anything here — pure byte relay keeps the shim
//! independent of protocol revisions.

use std::io::{Read as _, Write as _};
use std::os::unix::net::UnixStream;
use std::thread;

pub fn run(socket_path: &str) -> anyhow::Result<()> {
    let stream = UnixStream::connect(socket_path)
        .map_err(|e| anyhow::anyhow!("connect {socket_path}: {e}"))?;
    let stream_to_stdout = stream.try_clone()?;
    let mut stdin_to_socket = stream;

    // Thread A: stdin (from harness) → socket (to MCP server).
    let t_in = thread::Builder::new()
        .name("kittest-mcp-shim-stdin".into())
        .spawn(move || {
            let mut stdin = std::io::stdin().lock();
            let _ = std::io::copy(&mut stdin, &mut stdin_to_socket);
            // EOF on stdin or write error → shutdown write side so peer sees EOF.
            let _ = stdin_to_socket.shutdown(std::net::Shutdown::Write);
        })?;

    // Thread B: socket (from MCP server) → stdout (to harness).
    // Runs on main thread so the process exits when stdout closes.
    let mut stdout = std::io::stdout().lock();
    let mut buf = vec![0u8; 64 * 1024];
    let mut reader = stream_to_stdout;
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
