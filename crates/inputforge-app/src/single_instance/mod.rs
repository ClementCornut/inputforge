//! Per-user process ownership and authenticated loopback editor activation.
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    fs::{File, OpenOptions, TryLockError},
    io::{self, Read, Write},
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    path::Path,
    time::{Duration, Instant},
};
use uuid::Uuid;

const ACK: &[u8] = b"shown";
const ENDPOINT: &str = "instance.endpoint";

pub(crate) struct Instance {
    // Keep the lock file: unlinking it would allow two different locked inodes.
    _lock: File,
    socket: UdpSocket,
    request: String,
}

impl Instance {
    /// Returns an owner, or None after the running editor acknowledges Show.
    pub(crate) fn acquire(directory: &Path) -> io::Result<Option<Self>> {
        std::fs::create_dir_all(directory)?;
        let lock = open_private(&directory.join("instance.lock"), false)?;
        let endpoint = directory.join(ENDPOINT);
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match lock.try_lock() {
                Ok(()) => return Self::listen(lock, &endpoint).map(Some),
                Err(TryLockError::WouldBlock) => {}
                Err(TryLockError::Error(error)) => return Err(error),
            }
            if request_show(&endpoint).is_ok() {
                return Ok(None);
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "InputForge is running but did not acknowledge Show GUI",
                ));
            }
            // The owner may still be publishing its endpoint, or may exit.
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn listen(lock: File, endpoint: &Path) -> io::Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))?;
        socket.set_nonblocking(true)?;
        let token = Uuid::new_v4();
        write!(
            open_private(endpoint, true)?,
            "{}\n{token}",
            socket.local_addr()?
        )?;
        Ok(Self {
            _lock: lock,
            socket,
            request: format!("SHOW {token}"),
        })
    }

    /// Runs on the native GUI thread, including while the editor is hidden.
    pub(crate) fn process_pending(&self, mut show: impl FnMut()) -> io::Result<()> {
        let mut buffer = [0; 64];
        // Bound work per GUI tick even if another local process floods the port.
        for _ in 0..32 {
            match self.socket.recv_from(&mut buffer) {
                Ok((length, peer)) => {
                    if peer.ip().is_loopback() && &buffer[..length] == self.request.as_bytes() {
                        show();
                        self.socket.send_to(ACK, peer)?;
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}

fn open_private(path: &Path, truncate: bool) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(true)
        .create(true)
        .truncate(truncate);
    #[cfg(unix)]
    options.mode(0o600);
    options.open(path)
}

fn request_show(endpoint: &Path) -> io::Result<()> {
    let mut content = String::new();
    File::open(endpoint)?
        .take(128)
        .read_to_string(&mut content)?;
    let (address, token) = content
        .split_once('\n')
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing instance endpoint"))?;
    let address: SocketAddr = address
        .parse()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if !address.ip().is_loopback() || address.port() == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "instance endpoint must be loopback",
        ));
    }
    let token = Uuid::parse_str(token)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0))?;
    client.set_read_timeout(Some(Duration::from_millis(500)))?;
    client.connect(address)?;
    client.send(format!("SHOW {token}").as_bytes())?;
    let mut acknowledgement = [0; 64];
    let length = client.recv(&mut acknowledgement)?;
    if &acknowledgement[..length] != ACK {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid instance acknowledgement",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
