//! LAN P2P 번들 전송.
//!
//! 보내는 쪽이 TCP로 대기하고 짧은 페어링 코드를 보여 준다. 받는 쪽은 주소와 코드를 입력한다.
//! 코드는 짧기 때문에 SPAKE2로 세션 키를 합의해 도청자가 오프라인으로 코드를 맞춰 볼 수 없게 하고,
//! 키 확인을 마친 뒤에만 ChaCha20-Poly1305로 암호화한 번들을 보낸다.
//!
//! 프레임: u32 BE 길이 + 본문. 암호화 프레임의 nonce는 방향별 키 + 8바이트 카운터다.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use hkdf::Hkdf;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use spake2::{Ed25519Group, Identity, Password, Spake2};

use crate::error::{Result, SyncError};

pub const DISCOVERY_PORT: u16 = 47820;
const PROTOCOL_ID: &[u8] = b"claude-sync/p2p/v1";
const CODE_ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
const CODE_LENGTH: usize = 8;
const MAX_FAILED_ATTEMPTS: u32 = 3;
const MAX_HANDSHAKE_FRAME: usize = 4 * 1024;
const CHUNK_BYTES: usize = 256 * 1024;
const MAX_DATA_FRAME: usize = CHUNK_BYTES + 64;
pub const MAX_RECEIVE_BYTES: u64 = 1024 * 1024 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(30);

pub fn generate_code() -> String {
    let mut rng = rand::thread_rng();
    let raw: String = (0..CODE_LENGTH)
        .map(|_| CODE_ALPHABET[rng.gen_range(0..CODE_ALPHABET.len())] as char)
        .collect();
    format!("{}-{}", &raw[..4], &raw[4..])
}

/// 대소문자, 하이픈, 공백, 헷갈리는 글자(O→0 등)를 정리해 비교 가능한 형태로 만든다.
pub fn normalize_code(code: &str) -> String {
    code.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        })
        .collect()
}

struct SessionKeys {
    send: ChaCha20Poly1305,
    receive: ChaCha20Poly1305,
    send_counter: u64,
    receive_counter: u64,
    my_confirm: [u8; 32],
    peer_confirm: [u8; 32],
}

#[derive(Clone, Copy, PartialEq)]
enum Role {
    Sender,
    Receiver,
}

impl SessionKeys {
    fn derive(shared: &[u8], role: Role) -> Self {
        let hkdf = Hkdf::<Sha256>::new(Some(PROTOCOL_ID), shared);
        let expand = |info: &[u8]| {
            let mut out = [0u8; 32];
            hkdf.expand(info, &mut out).expect("32바이트는 HKDF 출력 길이로 유효하다");
            out
        };
        let sender_key = expand(b"sender->receiver key");
        let receiver_key = expand(b"receiver->sender key");
        let sender_confirm = expand(b"sender confirm");
        let receiver_confirm = expand(b"receiver confirm");
        let (send, receive, my_confirm, peer_confirm) = match role {
            Role::Sender => (sender_key, receiver_key, sender_confirm, receiver_confirm),
            Role::Receiver => (receiver_key, sender_key, receiver_confirm, sender_confirm),
        };
        Self {
            send: ChaCha20Poly1305::new(Key::from_slice(&send)),
            receive: ChaCha20Poly1305::new(Key::from_slice(&receive)),
            send_counter: 0,
            receive_counter: 0,
            my_confirm,
            peer_confirm,
        }
    }

    fn nonce(counter: u64) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        nonce[4..].copy_from_slice(&counter.to_be_bytes());
        nonce
    }

    fn seal(&mut self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let nonce = Self::nonce(self.send_counter);
        self.send_counter += 1;
        self.send
            .encrypt(Nonce::from_slice(&nonce), plaintext)
            .map_err(|_| SyncError::P2p("암호화 실패".into()))
    }

    fn open(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>> {
        let nonce = Self::nonce(self.receive_counter);
        self.receive_counter += 1;
        self.receive
            .decrypt(Nonce::from_slice(&nonce), ciphertext)
            .map_err(|_| SyncError::P2p("복호화 실패: 데이터가 손상되었거나 변조되었습니다".into()))
    }
}

fn write_frame(stream: &mut TcpStream, body: &[u8]) -> Result<()> {
    stream.write_all(&(body.len() as u32).to_be_bytes())?;
    stream.write_all(body)?;
    Ok(())
}

fn read_frame(stream: &mut TcpStream, max_len: usize) -> Result<Vec<u8>> {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes)?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    if len > max_len {
        return Err(SyncError::P2p(format!("프레임이 너무 큽니다 ({len} bytes)")));
    }
    let mut body = vec![0u8; len];
    stream.read_exact(&mut body)?;
    Ok(body)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// 양쪽이 SPAKE2 메시지를 교환한 뒤, 받는 쪽이 먼저 키 확인 값을 보내고 보내는 쪽이 검증한다.
/// 보내는 쪽은 받는 쪽을 확인한 뒤에만 자신의 확인 값을 보내므로, 틀린 코드로 접속한 상대는
/// 보내는 쪽에 대해 아무것도 얻지 못한다.
fn handshake(stream: &mut TcpStream, code: &str, role: Role) -> Result<SessionKeys> {
    let password = Password::new(normalize_code(code).as_bytes());
    let (state, outbound) = Spake2::<Ed25519Group>::start_symmetric(&password, &Identity::new(PROTOCOL_ID));
    write_frame(stream, &outbound)?;
    let inbound = read_frame(stream, MAX_HANDSHAKE_FRAME)?;
    let shared = state
        .finish(&inbound)
        .map_err(|e| SyncError::P2p(format!("키 교환 실패: {e:?}")))?;
    let keys = SessionKeys::derive(&shared, role);

    match role {
        Role::Receiver => {
            write_frame(stream, &keys.my_confirm)?;
            let peer = read_frame(stream, MAX_HANDSHAKE_FRAME)
                .map_err(|_| SyncError::P2p("페어링 코드가 틀렸습니다".into()))?;
            if !constant_time_eq(&peer, &keys.peer_confirm) {
                return Err(SyncError::P2p("상대를 확인할 수 없습니다 (코드 불일치)".into()));
            }
        }
        Role::Sender => {
            let peer = read_frame(stream, MAX_HANDSHAKE_FRAME)?;
            if !constant_time_eq(&peer, &keys.peer_confirm) {
                return Err(SyncError::P2p("페어링 코드가 틀렸습니다".into()));
            }
            write_frame(stream, &keys.my_confirm)?;
        }
    }
    Ok(keys)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransferHeader {
    device_name: String,
    file_name: String,
    size: u64,
    sha256: String,
    item_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Announcement {
    app: String,
    version: u32,
    device_name: String,
    port: u16,
    item_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShareInfo {
    pub code: String,
    pub port: u16,
    pub addresses: Vec<String>,
    pub device_name: String,
    pub item_count: usize,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SharePhase {
    Idle,
    Waiting,
    Transferring,
    Completed,
    Failed,
    Stopped,
}

impl SharePhase {
    pub fn is_finished(self) -> bool {
        matches!(self, SharePhase::Completed | SharePhase::Failed | SharePhase::Stopped)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ShareStatus {
    pub phase: SharePhase,
    pub message: String,
    pub peer: Option<String>,
    pub failed_attempts: u32,
}

impl ShareStatus {
    pub fn idle() -> Self {
        Self {
            phase: SharePhase::Idle,
            message: "공유 중이 아닙니다".into(),
            peer: None,
            failed_attempts: 0,
        }
    }
}

pub struct ShareOptions {
    pub port: u16,
    pub device_name: String,
    pub announce: bool,
    pub code: Option<String>,
}

/// 진행 중인 공유 세션. `stop()`하거나 drop하면 대기를 멈춘다.
pub struct ShareSession {
    pub info: ShareInfo,
    status: Arc<Mutex<ShareStatus>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl ShareSession {
    pub fn status(&self) -> ShareStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or_else(|_| ShareStatus::idle())
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }

    /// 끝날 때까지 기다린다. CLI에서 쓴다.
    pub fn wait(&mut self) -> ShareStatus {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.status()
    }
}

impl Drop for ShareSession {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn start_share(bundle_path: &Path, item_count: usize, options: ShareOptions) -> Result<ShareSession> {
    let bundle_bytes = std::fs::read(bundle_path)?;
    let listener = TcpListener::bind(("0.0.0.0", options.port))
        .or_else(|_| TcpListener::bind(("0.0.0.0", 0)))
        .map_err(|e| SyncError::P2p(format!("포트를 열 수 없습니다: {e}")))?;
    listener.set_nonblocking(true)?;
    let port = listener.local_addr()?.port();

    let code = options.code.clone().unwrap_or_else(generate_code);
    let info = ShareInfo {
        code: code.clone(),
        port,
        addresses: local_addresses().into_iter().map(|ip| format!("{ip}:{port}")).collect(),
        device_name: options.device_name.clone(),
        item_count,
        size_bytes: bundle_bytes.len() as u64,
    };
    let header = TransferHeader {
        device_name: options.device_name.clone(),
        file_name: bundle_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "bundle.zip".into()),
        size: bundle_bytes.len() as u64,
        sha256: format!("{:x}", Sha256::digest(&bundle_bytes)),
        item_count,
    };

    let status = Arc::new(Mutex::new(ShareStatus {
        phase: SharePhase::Waiting,
        message: "상대 기기의 연결을 기다리는 중".into(),
        peer: None,
        failed_attempts: 0,
    }));
    let stop = Arc::new(AtomicBool::new(false));

    if options.announce {
        spawn_announcer(
            Announcement {
                app: "claude-sync".into(),
                version: 1,
                device_name: options.device_name.clone(),
                port,
                item_count,
            },
            stop.clone(),
            status.clone(),
        );
    }

    let worker = {
        let status = status.clone();
        let stop = stop.clone();
        std::thread::spawn(move || serve(listener, &code, &header, &bundle_bytes, &status, &stop))
    };

    Ok(ShareSession {
        info,
        status,
        stop,
        worker: Some(worker),
    })
}

fn update(status: &Mutex<ShareStatus>, f: impl FnOnce(&mut ShareStatus)) {
    if let Ok(mut s) = status.lock() {
        f(&mut s);
    }
}

fn serve(
    listener: TcpListener,
    code: &str,
    header: &TransferHeader,
    bundle: &[u8],
    status: &Mutex<ShareStatus>,
    stop: &AtomicBool,
) {
    let mut failed_attempts = 0;
    loop {
        if stop.load(Ordering::SeqCst) {
            update(status, |s| {
                s.phase = SharePhase::Stopped;
                s.message = "공유를 중지했습니다".into();
            });
            return;
        }
        let (mut stream, peer) = match listener.accept() {
            Ok(conn) => conn,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(e) => {
                update(status, |s| {
                    s.phase = SharePhase::Failed;
                    s.message = format!("연결 대기 실패: {e}");
                });
                return;
            }
        };

        update(status, |s| {
            s.phase = SharePhase::Transferring;
            s.peer = Some(peer.to_string());
            s.message = format!("{peer} 와 페어링하는 중");
        });
        match send_to(&mut stream, code, header, bundle) {
            Ok(()) => {
                update(status, |s| {
                    s.phase = SharePhase::Completed;
                    s.message = format!("{peer} 로 전송을 마쳤습니다");
                });
                return;
            }
            Err(e) => {
                failed_attempts += 1;
                let exhausted = failed_attempts >= MAX_FAILED_ATTEMPTS;
                update(status, |s| {
                    s.failed_attempts = failed_attempts;
                    s.peer = None;
                    if exhausted {
                        s.phase = SharePhase::Failed;
                        s.message = format!("실패가 {MAX_FAILED_ATTEMPTS}번 반복되어 공유를 닫았습니다. 마지막 오류: {e}");
                    } else {
                        s.phase = SharePhase::Waiting;
                        s.message = format!("{peer} 연결 실패 ({e}). 다시 기다리는 중");
                    }
                });
                if exhausted {
                    return;
                }
            }
        }
    }
}

fn send_to(stream: &mut TcpStream, code: &str, header: &TransferHeader, bundle: &[u8]) -> Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    let mut keys = handshake(stream, code, Role::Sender)?;

    let header_frame = keys.seal(&serde_json::to_vec(header)?)?;
    write_frame(stream, &header_frame)?;
    for chunk in bundle.chunks(CHUNK_BYTES) {
        let frame = keys.seal(chunk)?;
        write_frame(stream, &frame)?;
    }
    let end = keys.seal(&[])?;
    write_frame(stream, &end)?;

    let ack = read_frame(stream, MAX_HANDSHAKE_FRAME)?;
    let ack = keys.open(&ack)?;
    if ack != b"ok" {
        return Err(SyncError::P2p(format!("상대가 수신을 거부했습니다: {}", String::from_utf8_lossy(&ack))));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct Received {
    pub path: PathBuf,
    pub device_name: String,
    pub size_bytes: u64,
    pub item_count: usize,
}

/// `address`("ip:port")의 공유에 접속해 번들을 `dest_dir`에 저장한다.
pub fn receive(address: &str, code: &str, dest_dir: &Path, progress: &dyn Fn(u64, u64)) -> Result<Received> {
    let socket_addr: SocketAddr = address
        .to_socket_addrs()
        .map_err(|e| SyncError::P2p(format!("주소를 해석할 수 없습니다 ({address}): {e}")))?
        .next()
        .ok_or_else(|| SyncError::P2p(format!("주소를 해석할 수 없습니다: {address}")))?;
    let mut stream = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(10))
        .map_err(|e| SyncError::P2p(format!("{address}에 연결할 수 없습니다: {e}")))?;
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;

    let mut keys = handshake(&mut stream, code, Role::Receiver)?;
    let header_frame = read_frame(&mut stream, MAX_DATA_FRAME)?;
    let header: TransferHeader = serde_json::from_slice(&keys.open(&header_frame)?)
        .map_err(|e| SyncError::P2p(format!("잘못된 전송 헤더: {e}")))?;
    if header.size > MAX_RECEIVE_BYTES {
        let refusal = keys.seal(b"too large")?;
        let _ = write_frame(&mut stream, &refusal);
        return Err(SyncError::P2p(format!("번들이 너무 큽니다 ({} bytes)", header.size)));
    }

    let mut data = Vec::with_capacity(header.size as usize);
    loop {
        let frame = read_frame(&mut stream, MAX_DATA_FRAME)?;
        let chunk = keys.open(&frame)?;
        if chunk.is_empty() {
            break;
        }
        data.extend_from_slice(&chunk);
        if data.len() as u64 > header.size {
            return Err(SyncError::P2p("선언한 크기보다 많은 데이터가 왔습니다".into()));
        }
        progress(data.len() as u64, header.size);
    }
    if data.len() as u64 != header.size || format!("{:x}", Sha256::digest(&data)) != header.sha256 {
        return Err(SyncError::P2p("받은 데이터의 무결성 검사에 실패했습니다".into()));
    }

    std::fs::create_dir_all(dest_dir)?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let device = sanitize_file_part(&header.device_name);
    let path = dest_dir.join(format!("{stamp}-from-{device}.zip"));
    crate::fsutil::atomic_write(&path, &data)?;

    let ack = keys.seal(b"ok")?;
    write_frame(&mut stream, &ack)?;
    Ok(Received {
        path,
        device_name: header.device_name,
        size_bytes: header.size,
        item_count: header.item_count,
    })
}

fn sanitize_file_part(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .take(40)
        .collect();
    if cleaned.is_empty() { "peer".into() } else { cleaned }
}

/// 외부로 패킷을 보내지 않고 기본 경로의 LAN IP를 알아낸다 (UDP connect는 라우팅만 정한다).
pub fn local_addresses() -> Vec<String> {
    let mut addresses = Vec::new();
    if let Ok(socket) = UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("192.0.2.1:9").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                if !addr.ip().is_unspecified() {
                    addresses.push(addr.ip().to_string());
                }
            }
        }
    }
    addresses.push("127.0.0.1".into());
    addresses
}

fn spawn_announcer(announcement: Announcement, stop: Arc<AtomicBool>, status: Arc<Mutex<ShareStatus>>) {
    std::thread::spawn(move || {
        let Ok(socket) = UdpSocket::bind("0.0.0.0:0") else {
            return;
        };
        if socket.set_broadcast(true).is_err() {
            return;
        }
        let Ok(message) = serde_json::to_vec(&announcement) else {
            return;
        };
        while !stop.load(Ordering::SeqCst) {
            let still_waiting = status.lock().map(|s| s.phase == SharePhase::Waiting).unwrap_or(false);
            if still_waiting {
                let _ = socket.send_to(&message, ("255.255.255.255", DISCOVERY_PORT));
            } else if status.lock().map(|s| s.phase.is_finished()).unwrap_or(true) {
                return;
            }
            std::thread::sleep(Duration::from_millis(1000));
        }
    });
}

#[derive(Debug, Clone, Serialize)]
pub struct PeerInfo {
    pub device_name: String,
    pub address: String,
    pub item_count: usize,
}

/// 같은 LAN에서 공유 중인 기기를 `timeout` 동안 찾는다. 방화벽이 브로드캐스트를 막으면 비어 있을 수 있다.
pub fn discover(timeout: Duration) -> Result<Vec<PeerInfo>> {
    let socket = UdpSocket::bind(("0.0.0.0", DISCOVERY_PORT))
        .map_err(|e| SyncError::P2p(format!("탐색 포트 {DISCOVERY_PORT}를 열 수 없습니다: {e}")))?;
    let deadline = Instant::now() + timeout;
    let mut peers: Vec<PeerInfo> = Vec::new();
    let mut buffer = [0u8; 2048];
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        if remaining.is_zero() {
            break;
        }
        socket.set_read_timeout(Some(remaining.min(Duration::from_millis(250))))?;
        let Ok((len, from)) = socket.recv_from(&mut buffer) else {
            continue;
        };
        let Ok(announcement) = serde_json::from_slice::<Announcement>(&buffer[..len]) else {
            continue;
        };
        if announcement.app != "claude-sync" {
            continue;
        }
        let address = format!("{}:{}", from.ip(), announcement.port);
        if !peers.iter().any(|p| p.address == address) {
            peers.push(PeerInfo {
                device_name: announcement.device_name,
                address,
                item_count: announcement.item_count,
            });
        }
    }
    Ok(peers)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("claude-sync-p2p-{label}-{}", rand::random::<u64>()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn share(dir: &Path, content: &[u8], code: &str) -> ShareSession {
        let bundle = dir.join("bundle.zip");
        std::fs::write(&bundle, content).unwrap();
        start_share(
            &bundle,
            3,
            ShareOptions {
                port: 0,
                device_name: "sender".into(),
                announce: false,
                code: Some(code.into()),
            },
        )
        .unwrap()
    }

    #[test]
    fn 올바른_코드로_받으면_같은_내용이_저장된다() {
        let dir = temp_dir("ok");
        let content: Vec<u8> = (0..700_000u32).map(|i| (i % 251) as u8).collect();
        let mut session = share(&dir, &content, "K7P3-QX9M");
        let address = format!("127.0.0.1:{}", session.info.port);

        let received = receive(&address, "k7p3qx9m", &dir.join("inbox"), &|_, _| {}).unwrap();

        assert_eq!(std::fs::read(&received.path).unwrap(), content);
        assert_eq!(received.device_name, "sender");
        assert_eq!(received.item_count, 3);
        assert_eq!(session.wait().phase, SharePhase::Completed);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn 틀린_코드는_거부되고_세번_틀리면_공유가_닫힌다() {
        let dir = temp_dir("bad");
        let mut session = share(&dir, b"data", "AAAA-BBBB");
        let address = format!("127.0.0.1:{}", session.info.port);

        for _ in 0..MAX_FAILED_ATTEMPTS {
            assert!(receive(&address, "CCCC-DDDD", &dir, &|_, _| {}).is_err());
        }
        let status = session.wait();

        assert_eq!(status.phase, SharePhase::Failed);
        assert_eq!(status.failed_attempts, MAX_FAILED_ATTEMPTS);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn 중지하면_대기가_끝난다() {
        let dir = temp_dir("stop");
        let mut session = share(&dir, b"data", "AAAA-BBBB");

        session.stop();

        assert_eq!(session.status().phase, SharePhase::Stopped);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn 코드_정규화는_대소문자와_구분자와_헷갈리는_글자를_무시한다() {
        assert_eq!(normalize_code("k7p3-qx9m"), "K7P3QX9M");
        assert_eq!(normalize_code("O0 Il"), "0011");
        assert_eq!(generate_code().len(), 9);
    }
}
