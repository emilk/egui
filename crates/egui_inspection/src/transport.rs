//! Cross-platform local-socket addressing for the inspection connection.
//!
//! [`interprocess`] maps a name to a unix domain socket (unix) or a named pipe (Windows),
//! so the transport works on every desktop platform without `cfg(unix)` gates. Both ends
//! must build the name the same way — hence the shared [`socket_name`] helper — and the
//! listener side allocates a fresh target via [`generate_socket_target`].

use std::io;

use interprocess::local_socket::{ListenerOptions, Name, prelude::*};
#[cfg(windows)]
use interprocess::local_socket::GenericNamespaced;
#[cfg(not(windows))]
use interprocess::local_socket::GenericFilePath;

/// The two halves of a connected local-socket stream, re-exported so consumers build
/// reader/writer threads without depending on `interprocess` directly.
pub use interprocess::local_socket::{RecvHalf, SendHalf};

/// Build a platform-appropriate local-socket [`Name`] from the env-var string produced by
/// [`generate_socket_target`].
///
/// On unix the string is a filesystem path (unix domain socket); on Windows it is a
/// namespaced identifier (named pipe). Both ends call this so they agree on the mapping.
///
/// # Errors
/// When the string is not a valid name for the platform's local-socket namespace.
pub fn socket_name(raw: &str) -> io::Result<Name<'static>> {
    #[cfg(not(windows))]
    {
        raw.to_owned().to_fs_name::<GenericFilePath>()
    }
    #[cfg(windows)]
    {
        raw.to_owned().to_ns_name::<GenericNamespaced>()
    }
}

/// A freshly-allocated local-socket target for the listener side.
pub struct SocketTarget {
    /// String to hand the peer (e.g. via an env var); parse it back with [`socket_name`].
    pub name: String,

    /// On unix, the tempdir owning the socket file — keep it alive for the socket's
    /// lifetime, then dropping it removes the file. Absent on Windows (named pipes have no
    /// filesystem object to clean up).
    #[cfg(not(windows))]
    #[expect(dead_code, reason = "RAII guard: kept alive to own the socket file")]
    dir: tempfile::TempDir,
}

/// Allocate a unique local-socket target for a listener to bind.
///
/// # Errors
/// On unix, when the backing tempdir can't be created.
pub fn generate_socket_target() -> io::Result<SocketTarget> {
    #[cfg(not(windows))]
    {
        let dir = tempfile::Builder::new()
            .prefix("egui-inspection-")
            .tempdir()?;
        let name = dir.path().join("inspection.sock").to_string_lossy().into_owned();
        Ok(SocketTarget { name, dir })
    }
    #[cfg(windows)]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let name = format!("egui-inspection-{}-{nonce}.sock", std::process::id());
        Ok(SocketTarget { name })
    }
}

/// Dial an already-listening inspection socket and split the stream into read / write halves.
///
/// The connector side of the connection: the live plugin, or the kittest harness when
/// [`crate::INSPECTION_SOCKET_ENV_VAR`] is set.
///
/// # Errors
/// When `raw` isn't a valid local-socket name, or the socket can't be dialed.
pub fn connect(raw: &str) -> io::Result<(RecvHalf, SendHalf)> {
    use interprocess::local_socket::Stream;
    let stream = Stream::connect(socket_name(raw)?)?;
    Ok(stream.split())
}

/// A bound synchronous local-socket listener — the listener side of the connection (kittest
/// harness in spawn mode, where it binds and then spawns an inspector pointed at the socket).
///
/// The MCP server uses the tokio listener directly; this sync wrapper exists for the
/// thread-based kittest harness.
pub struct Listener(interprocess::local_socket::Listener);

impl Listener {
    /// Bind a listener at the given target name (from [`generate_socket_target`]).
    ///
    /// # Errors
    /// When `raw` isn't a valid local-socket name, or the socket can't be bound.
    pub fn bind(raw: &str) -> io::Result<Self> {
        let listener = ListenerOptions::new().name(socket_name(raw)?).create_sync()?;
        Ok(Self(listener))
    }

    /// Block until a peer connects, then split the accepted stream into read / write halves.
    ///
    /// # Errors
    /// When accepting the inbound connection fails.
    pub fn accept(&self) -> io::Result<(RecvHalf, SendHalf)> {
        Ok(self.0.accept()?.split())
    }
}
