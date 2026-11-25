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
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    pub message: String,
}

#[cfg(windows)]
mod transport {
    use super::{Command, Response};
    use interprocess::{
        local_socket::{GenericNamespaced, ListenerOptions, Stream, prelude::*},
        os::windows::{local_socket::ListenerOptionsExt, security_descriptor::SecurityDescriptor},
    };
    use std::{
        hash::{DefaultHasher, Hash, Hasher},
        io::{self, BufRead, BufReader, Read, Write},
        sync::mpsc,
        thread,
    };
    use widestring::U16CString;

    pub type Request = (Command, mpsc::Sender<Response>);

    fn name() -> io::Result<String> {
        let path = crate::config::Config::default_path()?;
        let mut hash = DefaultHasher::new();
        path.hash(&mut hash);
        Ok(format!("mindswall-{:016x}", hash.finish()))
    }
    pub fn listen(tx: mpsc::Sender<Request>) -> io::Result<()> {
        let pipe_name = name()?;
        listen_name(&pipe_name, tx)
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
    pub fn send(command: Command) -> io::Result<Response> {
        let pipe_name = name()?;
        send_name(&pipe_name, command)
    }
    fn send_name(pipe_name: &str, command: Command) -> io::Result<Response> {
        let name = pipe_name.to_ns_name::<GenericNamespaced>()?;
        let mut stream = BufReader::new(Stream::connect(name).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("slideshow is not running or cannot be reached: {e}"),
            )
        })?);
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
