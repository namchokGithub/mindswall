use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    Next,
    Previous,
    Pause,
    Resume,
    Status,
    Reload,
    Set(std::path::PathBuf),
    /// Show the next image on one connected monitor that runs its own
    /// slideshow. Other monitors and the timer are not changed.
    NextMonitor(String),
    /// Go back to the previous image on one connected monitor that runs its
    /// own slideshow.
    PreviousMonitor(String),
    /// Ask a slideshow started from the CLI to stop so the desktop app can
    /// take over. The response carries the status to continue from.
    Handoff,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<PlaybackStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackStatus {
    pub state: String,
    pub current: Option<std::path::PathBuf>,
    pub position: Option<usize>,
    pub total: usize,
    /// Images combined with `current` in the shown wallpaper.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub combined: Vec<std::path::PathBuf>,
    /// The combined wallpaper file, when `current` is shown with other images.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<std::path::PathBuf>,
    /// Connected monitors that run their own slideshow.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub monitors: Vec<MonitorPlayback>,
}

/// The slideshow of one monitor with its own folders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorPlayback {
    pub monitor: String,
    pub current: Option<std::path::PathBuf>,
    pub position: Option<usize>,
    pub total: usize,
    /// Images combined with `current` in the shown wallpaper.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub combined: Vec<std::path::PathBuf>,
}

#[cfg(windows)]
mod transport {
    use super::{Command, Response};
    use interprocess::{
        local_socket::{GenericNamespaced, ListenerOptions, Stream, prelude::*},
        os::windows::{local_socket::ListenerOptionsExt, security_descriptor::SecurityDescriptor},
    };
    use std::{
        io::{self, BufRead, BufReader, Read, Write},
        os::windows::ffi::OsStrExt,
        sync::mpsc,
        thread,
    };
    use widestring::U16CString;
    use windows::Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{EqualSid, GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser},
        System::Threading::{
            GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
        },
    };

    pub type Request = (Command, mpsc::Sender<Response>);

    /// 64-bit FNV-1a. Unlike `DefaultHasher`, the result does not depend on
    /// the Rust version, so the CLI and the app always agree on the pipe name.
    pub(super) fn fnv1a(units: impl IntoIterator<Item = u16>) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for unit in units {
            for byte in unit.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        hash
    }

    fn name() -> io::Result<String> {
        let path = crate::config::Config::default_path()?;
        Ok(format!(
            "mindswall-{:016x}",
            fnv1a(path.as_os_str().encode_wide())
        ))
    }

    struct OwnedHandle(HANDLE);
    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            // SAFETY: the handle was opened by this module and is closed once.
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    /// The `TOKEN_USER` information of a process, in a buffer aligned for it.
    fn token_user(process: HANDLE) -> io::Result<Vec<u64>> {
        // SAFETY: the token handle is owned and closed by `OwnedHandle`; the
        // buffer is sized from the first call and aligned for `TOKEN_USER`.
        unsafe {
            let mut token = HANDLE::default();
            OpenProcessToken(process, TOKEN_QUERY, &mut token)?;
            let token = OwnedHandle(token);
            let mut length = 0u32;
            let _ = GetTokenInformation(token.0, TokenUser, None, 0, &mut length);
            let mut buffer = vec![0u64; (length as usize).div_ceil(8)];
            GetTokenInformation(
                token.0,
                TokenUser,
                Some(buffer.as_mut_ptr().cast()),
                length,
                &mut length,
            )?;
            Ok(buffer)
        }
    }

    /// Named pipe names are shared by every account on the machine, so
    /// another user could create the MindsWall pipe first. Only talk to a
    /// server that runs as the current user.
    fn verify_same_user(server_pid: u32) -> io::Result<()> {
        let denied = || {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "the slideshow control pipe is owned by another user",
            )
        };
        // SAFETY: the process handle is owned and closed by `OwnedHandle`, and
        // both buffers hold `TOKEN_USER` structures written by Windows.
        unsafe {
            let ours = token_user(GetCurrentProcess())?;
            let process = OwnedHandle(
                OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, server_pid)
                    .map_err(|_| denied())?,
            );
            let theirs = token_user(process.0).map_err(|_| denied())?;
            let ours = &*ours.as_ptr().cast::<TOKEN_USER>();
            let theirs = &*theirs.as_ptr().cast::<TOKEN_USER>();
            EqualSid(ours.User.Sid, theirs.User.Sid).map_err(|_| denied())
        }
    }

    fn connect_verified(pipe_name: &str) -> io::Result<Stream> {
        let name = pipe_name.to_ns_name::<GenericNamespaced>()?;
        let stream = Stream::connect(name).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("slideshow is not running or cannot be reached: {e}"),
            )
        })?;
        let server_pid = stream
            .peer_creds()?
            .pid()
            .ok_or_else(|| io::Error::other("slideshow process is unknown"))?;
        verify_same_user(server_pid)?;
        Ok(stream)
    }

    pub fn listen(tx: mpsc::Sender<Request>) -> io::Result<()> {
        let pipe_name = name()?;
        listen_name(&pipe_name, tx).map_err(|error| match connect_verified(&pipe_name) {
            Err(owner) if owner.kind() == io::ErrorKind::PermissionDenied => owner,
            _ => error,
        })
    }
    fn listen_name(pipe_name: &str, tx: mpsc::Sender<Request>) -> io::Result<()> {
        let name = pipe_name.to_ns_name::<GenericNamespaced>()?;
        let sddl = U16CString::from_str("D:P(A;;GA;;;OW)").map_err(io::Error::other)?;
        let descriptor = SecurityDescriptor::deserialize(&sddl)?;
        let listener = ListenerOptions::new()
            .name(name)
            .security_descriptor(descriptor)
            .create_sync()?;
        thread::spawn(move || {
            for connection in listener.incoming() {
                let Ok(connection) = connection else { continue };
                let tx = tx.clone();
                thread::spawn(move || {
                    let mut stream = BufReader::new(connection);
                    let mut line = String::new();
                    if (&mut stream).take(4097).read_line(&mut line).is_err() || line.len() > 4096 {
                        return;
                    }
                    let Ok(command) = serde_json::from_str::<Command>(&line) else {
                        return;
                    };
                    let (response_tx, response_rx) = mpsc::channel();
                    if tx.send((command, response_tx)).is_err() {
                        return;
                    }
                    if let Ok(response) = response_rx.recv() {
                        let _ = serde_json::to_writer(stream.get_mut(), &response);
                        let _ = stream.get_mut().write_all(b"\n");
                    }
                });
            }
        });
        Ok(())
    }
    /// How long a client waits for the running slideshow to answer. Long
    /// enough for a reload of a large collection, short enough that a stuck
    /// service cannot block callers indefinitely.
    const RESPONSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

    pub fn send(command: Command) -> io::Result<Response> {
        let pipe_name = name()?;
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = tx.send(send_name(&pipe_name, command));
        });
        rx.recv_timeout(RESPONSE_TIMEOUT).map_err(|_| {
            io::Error::new(io::ErrorKind::TimedOut, "slideshow did not respond in time")
        })?
    }
    fn send_name(pipe_name: &str, command: Command) -> io::Result<Response> {
        let mut stream = BufReader::new(connect_verified(pipe_name)?);
        serde_json::to_writer(stream.get_mut(), &command)?;
        stream.get_mut().write_all(b"\n")?;
        let mut line = String::new();
        stream.read_line(&mut line)?;
        serde_json::from_str(&line).map_err(io::Error::other)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn pipe_name_hash_is_stable() {
            assert_eq!(fnv1a([]), 0xcbf2_9ce4_8422_2325);
            assert_eq!(fnv1a("a".encode_utf16()), 0x089b_e207_b544_f1e4);
        }

        #[test]
        fn pipe_delivers_commands_and_prevents_second_listener() {
            let name = format!("mindswall-test-{}", std::process::id());
            let (tx, rx) = mpsc::channel();
            listen_name(&name, tx.clone()).unwrap();
            assert!(listen_name(&name, tx).is_err());
            let client_name = name.clone();
            let client = thread::spawn(move || send_name(&client_name, Command::Status).unwrap());
            let (command, reply) = rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
            assert!(matches!(command, Command::Status));
            reply
                .send(Response {
                    ok: true,
                    message: "running".into(),
                    status: None,
                })
                .unwrap();
            assert_eq!(client.join().unwrap().message, "running");
        }
    }
}
#[cfg(windows)]
pub use transport::{Request, listen, send};
#[cfg(not(windows))]
pub type Request = (Command, std::sync::mpsc::Sender<Response>);
#[cfg(not(windows))]
pub fn listen(_: std::sync::mpsc::Sender<Request>) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "slideshow IPC requires Windows",
    ))
}
#[cfg(not(windows))]
pub fn send(_: Command) -> std::io::Result<Response> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "slideshow IPC requires Windows",
    ))
}
