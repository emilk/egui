//! Cross-platform local-socket addressing for the inspection connection.
//!
//! [`interprocess`] maps a name to a unix domain socket (unix) or a named pipe (Windows),
//! so the transport works on every desktop platform without `cfg(unix)` gates. Both ends
//! must build the name the same way — hence the shared [`socket_name`] helper — and the
//! listener side allocates a fresh target via [`generate_socket_target`].

use std::io;

use interprocess::local_socket::Name;
#[cfg(windows)]
use interprocess::local_socket::{GenericNamespaced, ToNsName as _};
#[cfg(not(windows))]
use interprocess::local_socket::{GenericFilePath, ToFsName as _};

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
