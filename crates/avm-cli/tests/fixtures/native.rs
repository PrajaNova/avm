//! A mock foojay "disco" server and tar/zip helper, so native-plugin tests run offline.
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::TcpListener,
    path::Path,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

/// A local HTTP server answering the Java discovery API: the package list and its checksum.
pub struct Disco {
    pub url: String,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Disco {
    pub fn new(mut package: Value, checksum: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        package["links"]["pkg_info_uri"] = json!(format!("{url}/info"));
        let packages = json!({"result":[package]}).to_string();
        let info = json!({"result":[{"checksum":checksum,"checksum_type":"sha256"}]}).to_string();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let thread = thread::spawn(move || {
            while !stopped.load(Ordering::Relaxed) {
                if let Ok((mut stream, _)) = listener.accept() {
                    // Windows sockets inherit the listener's non-blocking mode; a
                    // read before the request lands would drop the connection.
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                        .unwrap();
                    let mut request = [0; 8192];
                    if let Ok(n) = stream.read(&mut request) {
                        let body = if request[..n].starts_with(b"GET /info ") {
                            &info
                        } else {
                            &packages
                        };
                        let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                    }
                } else {
                    thread::sleep(std::time::Duration::from_millis(5));
                }
            }
        });
        Self {
            url,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for Disco {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}

pub fn archive(root: &Path, name: &str, destination: &Path) {
    let mut cmd = Command::new("tar");
    if cfg!(windows) {
        cmd.args(["-a", "-cf"]);
    } else {
        cmd.arg("-czf");
    }
    assert!(cmd
        .arg(destination)
        .arg("-C")
        .arg(root)
        .arg(name)
        .status()
        .unwrap()
        .success());
}
