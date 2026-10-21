//! Per-job proxy: resolve every destination, then connect to the validated IP.
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use url::Url;

pub fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(matches!(a, 0 | 10 | 127 | 224..=255)
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192 && (b == 168 || b == 0 || (b == 88 && c == 99)))
                || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
                || (a == 203 && b == 0 && c == 113))
        }
        IpAddr::V6(ip) => {
            // Global unicast only; exclude protocol assignments, documentation,
            // transition/tunnel ranges and mapped IPv4 to keep one public rule.
            let s = ip.segments();
            (0x2000..=0x3fff).contains(&s[0])
                && !(s[0] == 0x2001 && (s[1] < 0x200 || s[1] == 0xdb8))
                && s[0] != 0x2002
                && !(s[0] == 0x3fff && s[1] < 0x1000)
        }
    }
}
pub fn web_url(text: &str) -> Result<Url, String> {
    if text.len() > 8192 || text.chars().any(|c| c.is_control()) {
        return Err("Use one HTTP or HTTPS URL under 8 KiB.".into());
    }
    let url = Url::parse(text.trim()).map_err(|_| "Enter a complete HTTP or HTTPS URL.")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url
            .port()
            .is_some_and(|p| p != if url.scheme() == "https" { 443 } else { 80 })
    {
        return Err(
            "Use a public HTTP(S) URL on its standard port, without a username or password.".into(),
        );
    }
    if let Some(host) = url.host() {
        let ip = match host {
            url::Host::Ipv4(i) => Some(IpAddr::V4(i)),
            url::Host::Ipv6(i) => Some(IpAddr::V6(i)),
            _ => None,
        };
        if ip.is_some_and(|i| !public_ip(i)) {
            return Err("Local and reserved network addresses are not supported.".into());
        }
    }
    Ok(url)
}
fn resolve(host: &str, port: u16) -> Result<Vec<SocketAddr>, String> {
    let addresses: Vec<_> = (host.trim_matches(['[', ']']), port)
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
        .collect();
    if addresses.is_empty() || addresses.iter().any(|a| !public_ip(a.ip())) {
        return Err("Destination must resolve only to public network addresses.".into());
    }
    Ok(addresses)
}
type Streams = Arc<Mutex<HashMap<u64, Vec<TcpStream>>>>;
pub struct Proxy {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    streams: Streams,
    thread: Option<JoinHandle<()>>,
}
impl Proxy {
    pub fn start() -> Result<Self, String> {
        let listener =
            TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).map_err(|e| e.to_string())?;
        let address = listener.local_addr().map_err(|e| e.to_string())?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let stop = Arc::new(AtomicBool::new(false));
        let streams: Streams = Arc::new(Mutex::new(HashMap::new()));
        let (halt, sockets) = (stop.clone(), streams.clone());
        let thread = thread::spawn(move || {
            let mut workers: Vec<JoinHandle<()>> = Vec::new();
            let mut next = 0;
            while !halt.load(Ordering::Relaxed) {
                let mut i = 0;
                while i < workers.len() {
                    if workers[i].is_finished() {
                        let _ = workers.swap_remove(i).join();
                    } else {
                        i += 1;
                    }
                }
                match listener.accept() {
                    Ok((mut client, _)) => {
                        if workers.len() >= 32 {
                            let _ =
                                client.write_all(b"HTTP/1.1 503 Busy\r\nConnection: close\r\n\r\n");
                            continue;
                        }
                        let id = next;
                        next += 1;
                        if let Ok(copy) = client.try_clone() {
                            sockets.lock().unwrap().insert(id, vec![copy]);
                        } else {
                            continue;
                        }
                        let (sockets, halt) = (sockets.clone(), halt.clone());
                        workers.push(thread::spawn(move || {
                            if forward(&mut client, id, &sockets, &halt).is_err() {
                                let _ = client.write_all(b"HTTP/1.1 502 Destination unavailable\r\nConnection: close\r\n\r\n");
                            }
                            sockets.lock().unwrap().remove(&id);
                        }));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(20))
                    }
                    Err(_) => break,
                }
            }
            for sockets in sockets.lock().unwrap().values() {
                for socket in sockets {
                    let _ = socket.shutdown(Shutdown::Both);
                }
            }
            for worker in workers {
                let _ = worker.join();
            }
        });
        Ok(Self {
            address,
            stop,
            streams,
            thread: Some(thread),
        })
    }
    pub fn url(&self) -> String {
        format!("http://{}", self.address)
    }
}
impl Drop for Proxy {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for sockets in self.streams.lock().unwrap().values() {
            for socket in sockets {
                let _ = socket.shutdown(Shutdown::Both);
            }
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn forward(
    client: &mut TcpStream,
    id: u64,
    streams: &Streams,
    stop: &AtomicBool,
) -> Result<(), String> {
    client
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|e| e.to_string())?;
    client
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    let mut byte = [0];
    let started = std::time::Instant::now();
    while !bytes.ends_with(b"\r\n\r\n") {
        if stop.load(Ordering::Relaxed) || started.elapsed() > Duration::from_secs(30) {
            return Err("Proxy request timed out".into());
        }
        if bytes.len() >= 16384 {
            return Err("Proxy headers exceed limit".into());
        }
        client.read_exact(&mut byte).map_err(|e| e.to_string())?;
        bytes.push(byte[0]);
    }
    let headers = std::str::from_utf8(&bytes).map_err(|_| "Invalid proxy headers")?;
    let mut lines = headers.split("\r\n");
    let mut first = lines
        .next()
        .ok_or("Missing proxy request")?
        .split_whitespace();
    let method = first.next().ok_or("Missing method")?;
    let target = first.next().ok_or("Missing destination")?;
    if !matches!(first.next(), Some("HTTP/1.1" | "HTTP/1.0")) || first.next().is_some() {
        return Err("Unsupported proxy request".into());
    }
    let tunnel = method == "CONNECT";
    let url = if tunnel {
        web_url(&format!("https://{target}"))?
    } else {
        web_url(target)?
    };
    if (!tunnel && url.scheme() != "http")
        || (tunnel && (url.path() != "/" || url.query().is_some() || url.fragment().is_some()))
    {
        return Err("Invalid proxy destination".into());
    }
    let mut upstream = None;
    for address in resolve(
        url.host_str().ok_or("Missing host")?,
        if tunnel { 443 } else { 80 },
    )? {
        if stop.load(Ordering::Relaxed) || started.elapsed() > Duration::from_secs(30) {
            return Err("Stopped".into());
        }
        if let Ok(socket) = TcpStream::connect_timeout(&address, Duration::from_secs(10)) {
            upstream = Some(socket);
            break;
        }
    }
    let mut upstream = upstream.ok_or("Cannot connect to destination")?;
    upstream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .map_err(|e| e.to_string())?;
    upstream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(|e| e.to_string())?;
    {
        let mut sockets = streams.lock().unwrap();
        if stop.load(Ordering::Relaxed) {
            return Err("Stopped".into());
        }
        sockets
            .get_mut(&id)
            .ok_or("Stopped")?
            .push(upstream.try_clone().map_err(|e| e.to_string())?);
    }
    if tunnel {
        client
            .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
            .map_err(|e| e.to_string())?;
    } else {
        let path = &url[url::Position::BeforePath..url::Position::AfterQuery];
        write!(
            upstream,
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
            url.host_str().unwrap()
        )
        .map_err(|e| e.to_string())?;
        for line in lines.filter(|l| !l.is_empty()) {
            let (name, _) = line.split_once(':').ok_or("Invalid header")?;
            if ![
                "host",
                "connection",
                "proxy-connection",
                "proxy-authorization",
            ]
            .iter()
            .any(|n| name.eq_ignore_ascii_case(n))
            {
                write!(upstream, "{line}\r\n").map_err(|e| e.to_string())?;
            }
        }
        upstream.write_all(b"\r\n").map_err(|e| e.to_string())?;
    }
    let mut incoming = client.try_clone().map_err(|e| e.to_string())?;
    let mut outgoing = upstream.try_clone().map_err(|e| e.to_string())?;
    let upload = thread::spawn(move || {
        let _ = std::io::copy(&mut incoming, &mut outgoing);
        let _ = outgoing.shutdown(Shutdown::Write);
    });
    let _ = std::io::copy(&mut upstream, client);
    let _ = client.shutdown(Shutdown::Both);
    let _ = upstream.shutdown(Shutdown::Both);
    let _ = upload.join();
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proxy_rejects_private_connections_and_owns_idle_sockets() {
        let proxy = Proxy::start().unwrap();
        let mut client = TcpStream::connect(proxy.address).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        client
            .write_all(b"CONNECT 127.0.0.1:443 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 502"));
        let mut idle = TcpStream::connect(proxy.address).unwrap();
        idle.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        drop(proxy);
        let mut byte = [0];
        assert!(matches!(idle.read(&mut byte), Ok(0) | Err(_)));
    }
    #[test]
    fn rejects_local_reserved_and_mapped_destinations() {
        for ip in [
            "0.0.0.0",
            "10.1.2.3",
            "127.0.0.1",
            "100.64.1.1",
            "169.254.1.1",
            "172.31.0.1",
            "192.168.0.1",
            "192.0.2.1",
            "198.18.1.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "::1",
            "::ffff:8.8.8.8",
            "fc00::1",
            "fe80::1",
            "2001:db8::1",
            "2002:808:808::1",
            "3fff::1",
        ] {
            assert!(!public_ip(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["8.8.8.8", "1.1.1.1", "2606:4700::1111"] {
            assert!(public_ip(ip.parse().unwrap()), "{ip}");
        }
    }
    #[test]
    fn urls_cannot_escape_public_web_ports() {
        for url in [
            "file:///C:/secret",
            "https://user:pass@example.com",
            "http://127.1",
            "https://[::1]",
            "https://example.com:444/",
            "https://example.com\nHost: local",
        ] {
            assert!(web_url(url).is_err(), "{url}");
        }
        assert!(web_url("https://example.com/watch?v=a").is_ok());
        assert!(resolve("localhost", 443).is_err());
    }
}
