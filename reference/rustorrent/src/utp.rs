use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

const UTP_VERSION: u8 = 1;
const UTP_HEADER_LEN: usize = 20;
const UTP_PAYLOAD_MAX: usize = 1200;
const UTP_ACK_TIMEOUT: Duration = Duration::from_millis(500);
const UTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const UTP_IDLE_TIMEOUT: Duration = Duration::from_secs(60);
const UTP_MAX_RETRANSMISSIONS: u8 = 8;
const INITIAL_CWND: usize = 4;
const MAX_CWND: usize = 64;
const MAX_CONNECTIONS: usize = 1024;
const MAX_PENDING_ACCEPTS: usize = 128;
const MAX_INBOUND_CONNECTIONS: usize = 256;
const MAX_INBOUND_CONNECTIONS_PER_IP: usize = 16;
const RECEIVE_BUFFER_BYTES: usize = 64 * 1024;
const RECEIVE_CHANNEL_CHUNKS: usize = 128;
const SEND_CHANNEL_PACKETS: usize = 64;
const MAX_RECEIVE_OVERFLOW_STRIKES: u8 = 8;

// LEDBAT constants (BEP 29 / RFC 6817)
const LEDBAT_TARGET_DELAY_US: i64 = 100_000; // 100ms target delay
const MAX_CWND_INCREASE: i64 = 3000; // max bytes gained per RTT
const BASE_DELAY_WINDOW: Duration = Duration::from_secs(120); // 2-minute rolling minimum

// SACK extension type
const EXT_SACK: u8 = 1;

const TYPE_DATA: u8 = 0;
const TYPE_FIN: u8 = 1;
const TYPE_STATE: u8 = 2;
const TYPE_RESET: u8 = 3;
const TYPE_SYN: u8 = 4;

#[derive(Clone)]
pub struct UtpConnector {
    cmd_tx: mpsc::Sender<Command>,
}

pub struct UtpListener {
    accept_rx: mpsc::Receiver<UtpStream>,
}

enum Command {
    Connect {
        addr: SocketAddr,
        resp: mpsc::Sender<Result<UtpStream, String>>,
    },
}

pub fn start(port: u16) -> (UtpConnector, UtpListener) {
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (accept_tx, accept_rx) = mpsc::sync_channel(MAX_PENDING_ACCEPTS);
    thread::spawn(move || {
        let socket = match UdpSocket::bind(("0.0.0.0", port)) {
            Ok(socket) => socket,
            Err(_) => match UdpSocket::bind((std::net::Ipv6Addr::UNSPECIFIED, port)) {
                Ok(socket) => socket,
                Err(_) => return,
            },
        };
        let _ = socket.set_read_timeout(Some(Duration::from_millis(50)));
        utp_loop(socket, cmd_rx, accept_tx);
    });
    (UtpConnector { cmd_tx }, UtpListener { accept_rx })
}

impl UtpConnector {
    pub fn connect(&self, addr: SocketAddr) -> Result<UtpStream, String> {
        let (resp_tx, resp_rx) = mpsc::channel();
        self.cmd_tx
            .send(Command::Connect {
                addr,
                resp: resp_tx,
            })
            .map_err(|_| "utp manager closed".to_string())?;
        resp_rx
            .recv()
            .map_err(|_| "utp connect failed".to_string())?
    }
}

impl UtpListener {
    pub fn try_accept(&self) -> Option<UtpStream> {
        self.accept_rx.try_recv().ok()
    }
}

pub struct UtpStream {
    #[allow(dead_code)]
    addr: SocketAddr,
    send_tx: mpsc::SyncSender<SendRequest>,
    recv_rx: mpsc::Receiver<Vec<u8>>,
    recv_budget: Arc<ReceiveBudget>,
    read_buf: VecDeque<u8>,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
}

struct ReceiveBudget {
    bytes: AtomicUsize,
    channel_chunks: AtomicUsize,
}

impl ReceiveBudget {
    fn new() -> Self {
        Self {
            bytes: AtomicUsize::new(0),
            channel_chunks: AtomicUsize::new(0),
        }
    }

    fn try_reserve_bytes(&self, amount: usize) -> bool {
        if amount == 0 || amount > RECEIVE_BUFFER_BYTES {
            return false;
        }
        self.bytes
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                current
                    .checked_add(amount)
                    .filter(|next| *next <= RECEIVE_BUFFER_BYTES)
            })
            .is_ok()
    }

    fn release_bytes(&self, amount: usize) {
        let _ = self
            .bytes
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                Some(current.saturating_sub(amount))
            });
    }

    fn try_reserve_channel_chunk(&self) -> bool {
        self.channel_chunks
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                (current < RECEIVE_CHANNEL_CHUNKS).then_some(current + 1)
            })
            .is_ok()
    }

    fn release_channel_chunk(&self) {
        let _ = self
            .channel_chunks
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                Some(current.saturating_sub(1))
            });
    }

    fn remaining_window(&self) -> usize {
        if self.channel_chunks.load(Ordering::Acquire) >= RECEIVE_CHANNEL_CHUNKS {
            return 0;
        }
        RECEIVE_BUFFER_BYTES.saturating_sub(self.bytes.load(Ordering::Acquire))
    }
}

impl UtpStream {
    #[allow(dead_code)]
    pub fn peer_addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn set_read_timeout(&mut self, timeout: Option<Duration>) {
        self.read_timeout = timeout;
    }

    pub fn set_write_timeout(&mut self, timeout: Option<Duration>) {
        self.write_timeout = timeout;
    }
}

impl Read for UtpStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        while self.read_buf.is_empty() {
            let received = match self.read_timeout {
                Some(timeout) => self.recv_rx.recv_timeout(timeout),
                None => self
                    .recv_rx
                    .recv()
                    .map_err(|_| mpsc::RecvTimeoutError::Disconnected),
            };
            match received {
                Ok(chunk) => {
                    self.recv_budget.release_channel_chunk();
                    self.read_buf.extend(chunk);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::WouldBlock,
                        "utp read timeout",
                    ));
                }
                Err(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "utp closed",
                    ));
                }
            }
        }
        let mut n = 0usize;
        while n < buf.len() {
            let Some(byte) = self.read_buf.pop_front() else {
                break;
            };
            buf[n] = byte;
            n += 1;
        }
        self.recv_budget.release_bytes(n);
        Ok(n)
    }
}

impl Drop for UtpStream {
    fn drop(&mut self) {
        self.recv_budget.release_bytes(self.read_buf.len());
        self.read_buf.clear();
        while let Ok(chunk) = self.recv_rx.try_recv() {
            self.recv_budget.release_channel_chunk();
            self.recv_budget.release_bytes(chunk.len());
        }
    }
}

impl Write for UtpStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut written = 0usize;
        for chunk in buf.chunks(UTP_PAYLOAD_MAX) {
            let (resp_tx, resp_rx) = mpsc::channel();
            let req = SendRequest {
                data: chunk.to_vec(),
                resp: resp_tx,
            };
            match self.send_tx.try_send(req) {
                Ok(()) => {}
                Err(mpsc::TrySendError::Full(_)) if written > 0 => return Ok(written),
                Err(mpsc::TrySendError::Full(_)) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::WouldBlock,
                        "utp send queue is full",
                    ));
                }
                Err(mpsc::TrySendError::Disconnected(_)) if written > 0 => return Ok(written),
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "utp send",
                    ));
                }
            }
            let response = match self.write_timeout {
                Some(timeout) => resp_rx.recv_timeout(timeout),
                None => resp_rx
                    .recv()
                    .map_err(|_| mpsc::RecvTimeoutError::Disconnected),
            };
            match response {
                Ok(Ok(())) => {
                    written += chunk.len();
                }
                Ok(Err(err)) => {
                    if written > 0 {
                        return Ok(written);
                    }
                    return Err(std::io::Error::other(err));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if written > 0 {
                        return Ok(written);
                    }
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::WouldBlock,
                        "utp write timeout",
                    ));
                }
                Err(_) => {
                    if written > 0 {
                        return Ok(written);
                    }
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "utp send failed",
                    ));
                }
            }
        }
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct SendRequest {
    data: Vec<u8>,
    resp: mpsc::Sender<Result<(), String>>,
}

struct ConnState {
    addr: SocketAddr,
    inbound: bool,
    send_id: u16,
    recv_id: Option<u16>,
    seq: u16,
    recv_seq: u16,
    state: ConnStatus,
    inflight: VecDeque<PendingPacket>,
    inflight_bytes: usize,
    cwnd: usize,
    peer_window: usize,
    send_rx: mpsc::Receiver<SendRequest>,
    recv_tx: mpsc::SyncSender<Vec<u8>>,
    recv_budget: Arc<ReceiveBudget>,
    last_advertised_window: usize,
    receive_overflow_strikes: u8,
    last_seen: Instant,
    connect_started: Instant,
    connect_resp: Option<mpsc::Sender<Result<UtpStream, String>>>,
    connect_stream: Option<UtpStream>,
    // Timestamp diff tracking (BEP 29)
    peer_timestamp: u32,
    timestamp_diff: u32,
    // Out-of-order received packets for SACK
    ooo_received: HashMap<u16, Vec<u8>>,
    // LEDBAT delay-based congestion state
    base_delay: Option<u32>,
    base_delay_updated: Instant,
    current_delay: u32,
}

#[derive(Default)]
struct SynLimiter {
    total: usize,
    by_ip: HashMap<IpAddr, usize>,
}

impl SynLimiter {
    fn normalized_ip(addr: SocketAddr) -> IpAddr {
        match addr.ip() {
            IpAddr::V6(ip) => ip
                .to_ipv4_mapped()
                .map(IpAddr::V4)
                .unwrap_or(IpAddr::V6(ip)),
            ip => ip,
        }
    }

    fn allows(&self, addr: SocketAddr) -> bool {
        self.total < MAX_INBOUND_CONNECTIONS
            && self
                .by_ip
                .get(&Self::normalized_ip(addr))
                .copied()
                .unwrap_or(0)
                < MAX_INBOUND_CONNECTIONS_PER_IP
    }

    fn opened(&mut self, addr: SocketAddr) {
        self.total = self.total.saturating_add(1);
        *self.by_ip.entry(Self::normalized_ip(addr)).or_default() += 1;
    }

    fn closed(&mut self, addr: SocketAddr) {
        self.total = self.total.saturating_sub(1);
        let ip = Self::normalized_ip(addr);
        if let Some(count) = self.by_ip.get_mut(&ip) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.by_ip.remove(&ip);
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ConnStatus {
    SynSent,
    Connected,
    Closed,
}

struct PendingPacket {
    seq: u16,
    data: Vec<u8>,
    sent_at: Instant,
    retransmissions: u8,
    resp: mpsc::Sender<Result<(), String>>,
}

fn utp_loop(
    socket: UdpSocket,
    cmd_rx: mpsc::Receiver<Command>,
    accept_tx: mpsc::SyncSender<UtpStream>,
) {
    let mut conns: HashMap<(SocketAddr, u16), ConnState> = HashMap::new();
    let mut syn_limiter = SynLimiter::default();
    let mut buf = [0u8; 1500];
    loop {
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                Command::Connect { addr, resp } => {
                    if conns.len() >= MAX_CONNECTIONS {
                        let _ = resp.send(Err("utp connection limit reached".to_string()));
                        continue;
                    }
                    let mut send_id = next_u16();
                    for _ in 0..u16::MAX {
                        if !conns.contains_key(&(addr, send_id)) {
                            break;
                        }
                        send_id = next_u16();
                    }
                    let seq = next_u16();
                    let (send_tx, send_rx) = mpsc::sync_channel(SEND_CHANNEL_PACKETS);
                    let (recv_tx, recv_rx) = mpsc::sync_channel(RECEIVE_CHANNEL_CHUNKS);
                    let recv_budget = Arc::new(ReceiveBudget::new());
                    let stream = UtpStream {
                        addr,
                        send_tx,
                        recv_rx,
                        recv_budget: Arc::clone(&recv_budget),
                        read_buf: VecDeque::new(),
                        read_timeout: None,
                        write_timeout: None,
                    };
                    let now = Instant::now();
                    let conn = ConnState {
                        addr,
                        inbound: false,
                        send_id,
                        recv_id: None,
                        seq,
                        recv_seq: 0,
                        state: ConnStatus::SynSent,
                        inflight: VecDeque::new(),
                        inflight_bytes: 0,
                        cwnd: INITIAL_CWND,
                        peer_window: RECEIVE_BUFFER_BYTES,
                        send_rx,
                        recv_tx,
                        recv_budget,
                        last_advertised_window: RECEIVE_BUFFER_BYTES,
                        receive_overflow_strikes: 0,
                        last_seen: now,
                        connect_started: now,
                        connect_resp: Some(resp),
                        connect_stream: Some(stream),
                        peer_timestamp: 0,
                        timestamp_diff: 0,
                        ooo_received: HashMap::new(),
                        base_delay: None,
                        base_delay_updated: now,
                        current_delay: 0,
                    };
                    let packet = build_packet(TYPE_SYN, send_id, seq, 0, &[]);
                    let _ = socket.send_to(&packet, addr);
                    conns.insert((addr, send_id), conn);
                }
            }
        }

        let mut remove_keys = Vec::new();
        for (key, conn) in conns.iter_mut() {
            if matches!(conn.state, ConnStatus::Closed) {
                remove_keys.push(*key);
                continue;
            }
            if conn.state == ConnStatus::SynSent
                && conn.connect_started.elapsed() >= UTP_CONNECT_TIMEOUT
            {
                drain_inflight_err(conn, "utp connect timeout");
                if let Some(resp) = conn.connect_resp.take() {
                    let _ = resp.send(Err("utp connect timeout".to_string()));
                }
                conn.connect_stream.take();
                conn.state = ConnStatus::Closed;
                remove_keys.push(*key);
                continue;
            }

            if conn.state == ConnStatus::Connected {
                let advanced = drain_contiguous_received(conn);
                if conn.state == ConnStatus::Closed {
                    drain_inflight_err(conn, "utp receive stream closed");
                    remove_keys.push(*key);
                    continue;
                }
                let receive_window = conn.recv_budget.remaining_window();
                let window_reopened = receive_window > conn.last_advertised_window
                    && (conn.last_advertised_window == 0
                        || receive_window - conn.last_advertised_window >= UTP_PAYLOAD_MAX
                        || receive_window == RECEIVE_BUFFER_BYTES);
                if advanced || window_reopened {
                    let sack = build_sack_extension(conn.recv_seq, &conn.ooo_received);
                    let state_packet = build_state_packet_with_sack(conn, &sack);
                    let _ = socket.send_to(&state_packet, conn.addr);
                }
            }

            // Fill send window from send_rx
            let mut stream_dropped = false;
            while conn.inflight.len() < conn.cwnd
                && conn.inflight_bytes < conn.peer_window
                && matches!(conn.state, ConnStatus::Connected)
            {
                match conn.send_rx.try_recv() {
                    Ok(req) => {
                        let data_len = req.data.len();
                        conn.seq = conn.seq.wrapping_add(1);
                        let receive_window = conn.recv_budget.remaining_window();
                        conn.last_advertised_window = receive_window;
                        let packet = build_packet_ext_with_window(
                            TYPE_DATA,
                            conn.send_id,
                            conn.seq,
                            conn.recv_seq,
                            conn.timestamp_diff,
                            receive_window,
                            &[],
                            &req.data,
                        );
                        let _ = socket.send_to(&packet, conn.addr);
                        conn.inflight.push_back(PendingPacket {
                            seq: conn.seq,
                            data: req.data,
                            sent_at: Instant::now(),
                            retransmissions: 0,
                            resp: req.resp,
                        });
                        conn.inflight_bytes = conn.inflight_bytes.saturating_add(data_len);
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        stream_dropped = true;
                        break;
                    }
                }
            }
            if stream_dropped {
                drain_inflight_err(conn, "utp stream dropped");
                let receive_window = conn.recv_budget.remaining_window();
                conn.last_advertised_window = receive_window;
                let fin = build_packet_ext_with_window(
                    TYPE_FIN,
                    conn.send_id,
                    conn.seq,
                    conn.recv_seq,
                    conn.timestamp_diff,
                    receive_window,
                    &[],
                    &[],
                );
                let _ = socket.send_to(&fin, conn.addr);
                conn.state = ConnStatus::Closed;
                remove_keys.push(*key);
                continue;
            }

            // Retransmit timed-out packets and halve cwnd (timeout fallback)
            let mut did_timeout = false;
            let mut delivery_failed = false;
            let mut remaining = VecDeque::with_capacity(conn.inflight.len());
            while let Some(mut pending) = conn.inflight.pop_front() {
                if pending.sent_at.elapsed() >= UTP_ACK_TIMEOUT {
                    if pending.retransmissions >= UTP_MAX_RETRANSMISSIONS {
                        let _ = pending
                            .resp
                            .send(Err("utp acknowledgement timeout".to_string()));
                        delivery_failed = true;
                        continue;
                    }
                    let receive_window = conn.recv_budget.remaining_window();
                    conn.last_advertised_window = receive_window;
                    let packet = build_packet_ext_with_window(
                        TYPE_DATA,
                        conn.send_id,
                        pending.seq,
                        conn.recv_seq,
                        conn.timestamp_diff,
                        receive_window,
                        &[],
                        &pending.data,
                    );
                    let _ = socket.send_to(&packet, conn.addr);
                    pending.sent_at = Instant::now();
                    pending.retransmissions += 1;
                    did_timeout = true;
                }
                remaining.push_back(pending);
            }
            conn.inflight = remaining;
            if delivery_failed {
                drain_inflight_err(conn, "utp acknowledgement timeout");
                conn.state = ConnStatus::Closed;
                remove_keys.push(*key);
                continue;
            }
            if did_timeout {
                conn.cwnd = (conn.cwnd / 2).max(1);
            }

            if conn.last_seen.elapsed() >= UTP_IDLE_TIMEOUT {
                drain_inflight_err(conn, "utp idle timeout");
                if let Some(resp) = conn.connect_resp.take() {
                    let _ = resp.send(Err("utp idle timeout".to_string()));
                }
                conn.connect_stream.take();
                conn.state = ConnStatus::Closed;
                remove_keys.push(*key);
            }
        }
        for key in remove_keys {
            if let Some(mut conn) = conns.remove(&key) {
                discard_receive_buffers(&mut conn);
                if conn.inbound {
                    syn_limiter.closed(conn.addr);
                }
            }
        }

        if let Ok((n, addr)) = socket.recv_from(&mut buf) {
            if n < UTP_HEADER_LEN {
                continue;
            }
            let Some(pkt) = parse_packet(&buf[..n]) else {
                continue;
            };
            let (ty, conn_id, seq, ack) = (pkt.ty, pkt.conn_id, pkt.seq, pkt.ack);
            let payload = pkt.payload;
            if ty == TYPE_SYN {
                let recv_id = conn_id;
                let send_id = conn_id.wrapping_add(1);
                if !payload.is_empty() {
                    let reset = build_packet(TYPE_RESET, send_id, 0, seq, &[]);
                    let _ = socket.send_to(&reset, addr);
                    continue;
                }
                if let Some(existing) = conns.get(&(addr, recv_id)) {
                    let state_pkt = build_packet_ext_with_window(
                        TYPE_STATE,
                        existing.send_id,
                        existing.seq,
                        existing.recv_seq,
                        existing.timestamp_diff,
                        existing.recv_budget.remaining_window(),
                        &[],
                        &[],
                    );
                    let _ = socket.send_to(&state_pkt, addr);
                    continue;
                }
                if conns.len() >= MAX_CONNECTIONS || !syn_limiter.allows(addr) {
                    let reset = build_packet(TYPE_RESET, send_id, 0, seq, &[]);
                    let _ = socket.send_to(&reset, addr);
                    continue;
                }
                let (send_tx, send_rx) = mpsc::sync_channel(SEND_CHANNEL_PACKETS);
                let (recv_tx, recv_rx) = mpsc::sync_channel(RECEIVE_CHANNEL_CHUNKS);
                let recv_budget = Arc::new(ReceiveBudget::new());
                let stream = UtpStream {
                    addr,
                    send_tx,
                    recv_rx,
                    recv_budget: Arc::clone(&recv_budget),
                    read_buf: VecDeque::new(),
                    read_timeout: None,
                    write_timeout: None,
                };
                let now = Instant::now();
                let mut conn = ConnState {
                    addr,
                    inbound: true,
                    send_id,
                    recv_id: Some(recv_id),
                    seq: next_u16(),
                    recv_seq: seq,
                    state: ConnStatus::Connected,
                    inflight: VecDeque::new(),
                    inflight_bytes: 0,
                    cwnd: INITIAL_CWND,
                    peer_window: pkt.window_size,
                    send_rx,
                    recv_tx,
                    recv_budget,
                    last_advertised_window: RECEIVE_BUFFER_BYTES,
                    receive_overflow_strikes: 0,
                    last_seen: now,
                    connect_started: now,
                    connect_resp: None,
                    connect_stream: None,
                    peer_timestamp: pkt.timestamp,
                    timestamp_diff: timestamp().wrapping_sub(pkt.timestamp),
                    ooo_received: HashMap::new(),
                    base_delay: None,
                    base_delay_updated: now,
                    current_delay: 0,
                };
                let state_pkt = build_state_packet(&mut conn);
                if accept_tx.try_send(stream).is_ok() {
                    let _ = socket.send_to(&state_pkt, addr);
                    conns.insert((addr, recv_id), conn);
                    syn_limiter.opened(addr);
                } else {
                    let reset = build_packet(TYPE_RESET, send_id, 0, seq, &[]);
                    let _ = socket.send_to(&reset, addr);
                }
                continue;
            }

            let key = (addr, conn_id);
            if !conns.contains_key(&key) && ty == TYPE_STATE {
                let mut match_key = None;
                for (conn_key, conn) in conns.iter() {
                    if conn_key.0 != addr {
                        continue;
                    }
                    if conn.state == ConnStatus::SynSent && conn.send_id.wrapping_add(1) == conn_id
                    {
                        match_key = Some(*conn_key);
                        break;
                    }
                }
                if let Some(old_key) = match_key {
                    if let Some(mut conn) = conns.remove(&old_key) {
                        let recv_id = conn_id;
                        conn.recv_id = Some(recv_id);
                        conn.state = ConnStatus::Connected;
                        conn.recv_seq = seq;
                        conn.peer_timestamp = pkt.timestamp;
                        conn.timestamp_diff = timestamp().wrapping_sub(pkt.timestamp);
                        conn.peer_window = pkt.window_size;
                        if pkt.timestamp_diff != 0 {
                            conn.current_delay = pkt.timestamp_diff;
                        }
                        if let Some(resp) = conn.connect_resp.take() {
                            if let Some(stream) = conn.connect_stream.take() {
                                let _ = resp.send(Ok(stream));
                            } else {
                                let _ = resp.send(Err("utp connect failed".to_string()));
                            }
                        }
                        let new_key = (addr, recv_id);
                        conns.insert(new_key, conn);
                    }
                }
                continue;
            }

            let conn = match conns.get_mut(&key) {
                Some(conn) => conn,
                None => continue,
            };
            if ty == TYPE_DATA && conn.state != ConnStatus::Connected {
                let reset = build_packet(TYPE_RESET, conn.send_id, conn.seq, conn.recv_seq, &[]);
                let _ = socket.send_to(&reset, conn.addr);
                drain_inflight_err(conn, "utp data arrived before connection handshake");
                conn.connect_stream.take();
                conn.state = ConnStatus::Closed;
                continue;
            }
            conn.last_seen = Instant::now();
            conn.peer_window = pkt.window_size;

            // Record peer timestamp and compute timestamp_diff for next outgoing packet
            conn.peer_timestamp = pkt.timestamp;
            conn.timestamp_diff = timestamp().wrapping_sub(pkt.timestamp);
            // Record the peer's reported delay for LEDBAT
            if pkt.timestamp_diff != 0 {
                conn.current_delay = pkt.timestamp_diff;
            }

            match ty {
                TYPE_STATE => {
                    // Ignore acknowledgements for sequence numbers we have not sent.
                    if ack != conn.seq && is_seq_before_or_equal(conn.seq, ack) {
                        continue;
                    }
                    // Cumulative ACK: remove all packets with seq <= ack
                    let mut bytes_acked: usize = 0;
                    while let Some(front) = conn.inflight.front() {
                        if front.seq == ack || is_seq_before_or_equal(front.seq, ack) {
                            let Some(p) = conn.inflight.pop_front() else {
                                break;
                            };
                            bytes_acked += p.data.len();
                            conn.inflight_bytes = conn.inflight_bytes.saturating_sub(p.data.len());
                            let _ = p.resp.send(Ok(()));
                        } else {
                            break;
                        }
                    }

                    // Process incoming SACK: mark selectively-acked inflight packets
                    if !pkt.sack.is_empty() {
                        let mut sack_acked = HashSet::new();
                        for (byte_idx, &byte) in pkt.sack.iter().enumerate() {
                            for bit in 0..8u16 {
                                if byte & (1 << bit) != 0 {
                                    let sacked_seq =
                                        ack.wrapping_add(2).wrapping_add(byte_idx as u16 * 8 + bit);
                                    sack_acked.insert(sacked_seq);
                                }
                            }
                        }
                        // Remove SACK-ed inflight packets and complete them
                        let mut remaining = VecDeque::new();
                        while let Some(p) = conn.inflight.pop_front() {
                            if sack_acked.contains(&p.seq) {
                                bytes_acked += p.data.len();
                                conn.inflight_bytes =
                                    conn.inflight_bytes.saturating_sub(p.data.len());
                                let _ = p.resp.send(Ok(()));
                            } else {
                                remaining.push_back(p);
                            }
                        }
                        conn.inflight = remaining;
                    }

                    // LEDBAT congestion control instead of simple additive increase
                    if bytes_acked > 0 {
                        ledbat_update_cwnd(conn, bytes_acked);
                    }
                }
                TYPE_DATA => {
                    match handle_data_packet(conn, seq, payload) {
                        DataPacketOutcome::State => {
                            // The cumulative ACK remains at the last payload
                            // actually admitted to the bounded receive path.
                            let sack_ext = build_sack_extension(conn.recv_seq, &conn.ooo_received);
                            let state_pkt = build_state_packet_with_sack(conn, &sack_ext);
                            let _ = socket.send_to(&state_pkt, conn.addr);
                        }
                        DataPacketOutcome::Reset => {
                            drain_inflight_err(conn, "utp receive budget exceeded");
                            let reset = build_packet_ext_with_window(
                                TYPE_RESET,
                                conn.send_id,
                                conn.seq,
                                conn.recv_seq,
                                conn.timestamp_diff,
                                0,
                                &[],
                                &[],
                            );
                            let _ = socket.send_to(&reset, conn.addr);
                        }
                    }
                }
                TYPE_FIN => {
                    if seq == conn.recv_seq.wrapping_add(1) {
                        conn.recv_seq = seq;
                    }
                    drain_inflight_err(conn, "utp closed");
                    if let Some(resp) = conn.connect_resp.take() {
                        let _ = resp.send(Err("utp closed".to_string()));
                    }
                    conn.connect_stream.take();
                    conn.state = ConnStatus::Closed;
                    let state_pkt = build_state_packet(conn);
                    let _ = socket.send_to(&state_pkt, conn.addr);
                }
                TYPE_RESET => {
                    drain_inflight_err(conn, "utp reset");
                    if let Some(resp) = conn.connect_resp.take() {
                        let _ = resp.send(Err("utp reset".to_string()));
                    }
                    conn.connect_stream.take();
                    conn.state = ConnStatus::Closed;
                }
                _ => {}
            }
        }
    }
}

fn build_state_packet(conn: &mut ConnState) -> Vec<u8> {
    build_state_packet_with_sack(conn, &[])
}

fn build_state_packet_with_sack(conn: &mut ConnState, sack: &[u8]) -> Vec<u8> {
    let receive_window = conn.recv_budget.remaining_window();
    conn.last_advertised_window = receive_window;
    build_packet_ext_with_window(
        TYPE_STATE,
        conn.send_id,
        conn.seq,
        conn.recv_seq,
        conn.timestamp_diff,
        receive_window,
        sack,
        &[],
    )
}

fn drain_inflight_err(conn: &mut ConnState, msg: &str) {
    while let Some(pkt) = conn.inflight.pop_front() {
        let _ = pkt.resp.send(Err(msg.to_string()));
    }
    conn.inflight_bytes = 0;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DataPacketOutcome {
    State,
    Reset,
}

enum DeliveryResult {
    Delivered,
    Full(Vec<u8>),
    Closed,
}

fn deliver_reserved_payload(conn: &mut ConnState, data: Vec<u8>) -> DeliveryResult {
    if !conn.recv_budget.try_reserve_channel_chunk() {
        return DeliveryResult::Full(data);
    }
    match conn.recv_tx.try_send(data) {
        Ok(()) => DeliveryResult::Delivered,
        Err(mpsc::TrySendError::Full(data)) => {
            conn.recv_budget.release_channel_chunk();
            DeliveryResult::Full(data)
        }
        Err(mpsc::TrySendError::Disconnected(data)) => {
            conn.recv_budget.release_channel_chunk();
            conn.recv_budget.release_bytes(data.len());
            DeliveryResult::Closed
        }
    }
}

fn drain_contiguous_received(conn: &mut ConnState) -> bool {
    let mut advanced = false;
    loop {
        let next = conn.recv_seq.wrapping_add(1);
        let Some(data) = conn.ooo_received.remove(&next) else {
            break;
        };
        match deliver_reserved_payload(conn, data) {
            DeliveryResult::Delivered => {
                conn.recv_seq = next;
                advanced = true;
            }
            DeliveryResult::Full(data) => {
                conn.ooo_received.insert(next, data);
                break;
            }
            DeliveryResult::Closed => {
                conn.state = ConnStatus::Closed;
                break;
            }
        }
    }
    advanced
}

fn receive_overflow(conn: &mut ConnState) -> DataPacketOutcome {
    conn.receive_overflow_strikes = conn.receive_overflow_strikes.saturating_add(1);
    if conn.receive_overflow_strikes >= MAX_RECEIVE_OVERFLOW_STRIKES {
        conn.state = ConnStatus::Closed;
        DataPacketOutcome::Reset
    } else {
        DataPacketOutcome::State
    }
}

fn handle_data_packet(conn: &mut ConnState, seq: u16, payload: &[u8]) -> DataPacketOutcome {
    if payload.is_empty() || payload.len() > UTP_PAYLOAD_MAX {
        conn.state = ConnStatus::Closed;
        return DataPacketOutcome::Reset;
    }

    drain_contiguous_received(conn);
    if conn.state == ConnStatus::Closed {
        return DataPacketOutcome::Reset;
    }
    if seq == conn.recv_seq {
        return DataPacketOutcome::State;
    }

    let expected = conn.recv_seq.wrapping_add(1);
    let offset = seq.wrapping_sub(expected);
    if offset > 32 {
        // Old duplicates and packets outside the SACK window are ignored.
        return if conn.recv_budget.remaining_window() == 0 {
            receive_overflow(conn)
        } else {
            DataPacketOutcome::State
        };
    }
    if conn.ooo_received.contains_key(&seq) {
        return if conn.recv_budget.remaining_window() == 0 {
            receive_overflow(conn)
        } else {
            DataPacketOutcome::State
        };
    }
    if conn.recv_budget.remaining_window() == 0 {
        return receive_overflow(conn);
    }
    if !conn.recv_budget.try_reserve_bytes(payload.len()) {
        return receive_overflow(conn);
    }
    conn.ooo_received.insert(seq, payload.to_vec());
    conn.receive_overflow_strikes = 0;
    if seq == expected {
        drain_contiguous_received(conn);
        if conn.state == ConnStatus::Closed {
            return DataPacketOutcome::Reset;
        }
    }
    DataPacketOutcome::State
}

fn discard_receive_buffers(conn: &mut ConnState) {
    let bytes = conn.ooo_received.drain().map(|(_, data)| data.len()).sum();
    conn.recv_budget.release_bytes(bytes);
}

fn is_seq_before_or_equal(seq: u16, ack: u16) -> bool {
    // Handle wrapping: seq is before or equal to ack if the difference is small
    let diff = ack.wrapping_sub(seq);
    diff < 0x8000
}

fn build_packet(ty: u8, conn_id: u16, seq: u16, ack: u16, payload: &[u8]) -> Vec<u8> {
    build_packet_ext(ty, conn_id, seq, ack, 0, &[], payload)
}

fn build_packet_ext(
    ty: u8,
    conn_id: u16,
    seq: u16,
    ack: u16,
    ts_diff: u32,
    extensions: &[u8],
    payload: &[u8],
) -> Vec<u8> {
    build_packet_ext_with_window(
        ty,
        conn_id,
        seq,
        ack,
        ts_diff,
        RECEIVE_BUFFER_BYTES,
        extensions,
        payload,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_packet_ext_with_window(
    ty: u8,
    conn_id: u16,
    seq: u16,
    ack: u16,
    ts_diff: u32,
    receive_window: usize,
    extensions: &[u8],
    payload: &[u8],
) -> Vec<u8> {
    let has_ext = !extensions.is_empty();
    let mut out = Vec::with_capacity(UTP_HEADER_LEN + extensions.len() + payload.len());
    out.push((ty << 4) | UTP_VERSION);
    // next-extension byte: 0 = no extensions, 1 = SACK follows
    out.push(if has_ext { EXT_SACK } else { 0 });
    out.extend_from_slice(&conn_id.to_be_bytes());
    out.extend_from_slice(&timestamp().to_be_bytes());
    out.extend_from_slice(&ts_diff.to_be_bytes());
    out.extend_from_slice(&(receive_window.min(u32::MAX as usize) as u32).to_be_bytes());
    out.extend_from_slice(&seq.to_be_bytes());
    out.extend_from_slice(&ack.to_be_bytes());
    out.extend_from_slice(extensions);
    out.extend_from_slice(payload);
    out
}

struct ParsedPacket<'a> {
    ty: u8,
    conn_id: u16,
    timestamp: u32,
    timestamp_diff: u32,
    window_size: usize,
    seq: u16,
    ack: u16,
    sack: Vec<u8>,
    payload: &'a [u8],
}

fn parse_packet(data: &[u8]) -> Option<ParsedPacket<'_>> {
    if data.len() < UTP_HEADER_LEN || data[0] & 0x0f != UTP_VERSION || data[0] >> 4 > TYPE_SYN {
        return None;
    }
    let ty = data[0] >> 4;
    let ext_type = data[1];
    let conn_id = u16::from_be_bytes([data[2], data[3]]);
    let ts = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    let ts_diff = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
    let window_size = u32::from_be_bytes([data[12], data[13], data[14], data[15]]) as usize;
    let seq = u16::from_be_bytes([data[16], data[17]]);
    let ack = u16::from_be_bytes([data[18], data[19]]);

    // Walk extensions chain starting after the 20-byte header
    let mut sack = Vec::new();
    let mut offset = UTP_HEADER_LEN;
    let mut cur_ext = ext_type;
    while cur_ext != 0 {
        if offset.checked_add(2)? > data.len() {
            return None;
        }
        let next = data[offset];
        let ext_len = data[offset + 1] as usize;
        let data_start = offset.checked_add(2)?;
        let data_end = data_start.checked_add(ext_len)?;
        if data_end > data.len() {
            return None;
        }
        if cur_ext == EXT_SACK {
            if ext_len == 0 || !ext_len.is_multiple_of(4) {
                return None;
            }
            sack.extend_from_slice(&data[data_start..data_end]);
        }
        offset = data_end;
        cur_ext = next;
    }

    let payload = &data[offset..];

    Some(ParsedPacket {
        ty,
        conn_id,
        timestamp: ts,
        timestamp_diff: ts_diff,
        window_size,
        seq,
        ack,
        sack,
        payload,
    })
}

/// Build SACK extension bytes: [next_ext=0, length, bitmask...]
/// The bitmask indicates which packets after `ack_nr` have been received.
fn build_sack_extension(ack_nr: u16, ooo: &HashMap<u16, Vec<u8>>) -> Vec<u8> {
    if ooo.is_empty() {
        return Vec::new();
    }
    // SACK bitmask: bit i represents ack_nr + 2 + i  (ack_nr+1 is the first missing)
    // BEP 29 uses 4 bytes (32 bits) as the standard SACK length
    let sack_len: usize = 4;
    let mut bitmask = vec![0u8; sack_len];
    for &s in ooo.keys() {
        let offset = s.wrapping_sub(ack_nr.wrapping_add(2));
        let bit_idx = offset as usize;
        if bit_idx < sack_len * 8 {
            bitmask[bit_idx / 8] |= 1 << (bit_idx % 8);
        }
    }
    // Extension header: next_extension=0, length, then bitmask
    let mut ext = Vec::with_capacity(2 + sack_len);
    ext.push(0); // no further extensions
    ext.push(sack_len as u8);
    ext.extend_from_slice(&bitmask);
    ext
}

/// Update LEDBAT congestion window based on delay measurement.
fn ledbat_update_cwnd(conn: &mut ConnState, bytes_acked: usize) {
    if conn.current_delay == 0 {
        return;
    }
    let now = Instant::now();
    // Maintain base_delay as min over last 2 minutes
    match conn.base_delay {
        Some(bd) if now.duration_since(conn.base_delay_updated) < BASE_DELAY_WINDOW => {
            if conn.current_delay < bd {
                conn.base_delay = Some(conn.current_delay);
            }
        }
        _ => {
            conn.base_delay = Some(conn.current_delay);
            conn.base_delay_updated = now;
        }
    }
    let base = conn.base_delay.unwrap_or(conn.current_delay);
    let queuing_delay = (conn.current_delay as i64).saturating_sub(base as i64);
    // LEDBAT formula: cwnd += (TARGET - queuing_delay) / TARGET * MAX_CWND_INCREASE / cwnd
    // We work in packet-count units; approximate packet size as UTP_PAYLOAD_MAX
    let cwnd_bytes = (conn.cwnd as i64) * (UTP_PAYLOAD_MAX as i64);
    if cwnd_bytes <= 0 {
        return;
    }
    let off_target = LEDBAT_TARGET_DELAY_US - queuing_delay;
    let gain = (off_target * MAX_CWND_INCREASE) / LEDBAT_TARGET_DELAY_US;
    // Scale by acked bytes / cwnd_bytes
    let acked = bytes_acked.max(1) as i64;
    let delta_bytes = (gain * acked) / cwnd_bytes;
    let delta_pkts = delta_bytes / (UTP_PAYLOAD_MAX as i64);
    let new_cwnd = (conn.cwnd as i64 + delta_pkts).clamp(1, MAX_CWND as i64);
    conn.cwnd = new_cwnd as usize;
}

fn timestamp() -> u32 {
    use std::sync::OnceLock;
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    let epoch = EPOCH.get_or_init(Instant::now);
    epoch.elapsed().as_micros() as u32
}

fn next_u16() -> u16 {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::OnceLock;
    static INIT: OnceLock<()> = OnceLock::new();
    static SEED: AtomicU32 = AtomicU32::new(0x1234_5678);
    INIT.get_or_init(|| {
        SEED.store(crate::system_entropy_u64() as u32, Ordering::Relaxed);
    });
    let mut x = SEED.load(Ordering::Relaxed);
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    SEED.store(x, Ordering::Relaxed);
    x as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receive_test_conn(recv_seq: u16) -> (ConnState, mpsc::Receiver<Vec<u8>>) {
        let (_send_tx, send_rx) = mpsc::channel();
        let (recv_tx, recv_rx) = mpsc::sync_channel(RECEIVE_CHANNEL_CHUNKS);
        let now = Instant::now();
        (
            ConnState {
                addr: "127.0.0.1:1".parse().unwrap(),
                inbound: true,
                send_id: 1,
                recv_id: Some(2),
                seq: 10,
                recv_seq,
                state: ConnStatus::Connected,
                inflight: VecDeque::new(),
                inflight_bytes: 0,
                cwnd: INITIAL_CWND,
                peer_window: RECEIVE_BUFFER_BYTES,
                send_rx,
                recv_tx,
                recv_budget: Arc::new(ReceiveBudget::new()),
                last_advertised_window: RECEIVE_BUFFER_BYTES,
                receive_overflow_strikes: 0,
                last_seen: now,
                connect_started: now,
                connect_resp: None,
                connect_stream: None,
                peer_timestamp: 0,
                timestamp_diff: 0,
                ooo_received: HashMap::new(),
                base_delay: None,
                base_delay_updated: now,
                current_delay: 0,
            },
            recv_rx,
        )
    }

    #[test]
    fn packet_roundtrip_preserves_fields() {
        let payload = b"hello-utp";
        let packet = build_packet(TYPE_DATA, 42, 100, 99, payload);
        assert_eq!(packet.len(), UTP_HEADER_LEN + payload.len());

        let pkt = parse_packet(&packet).unwrap();
        assert_eq!(pkt.ty, TYPE_DATA);
        assert_eq!(pkt.conn_id, 42);
        assert_eq!(pkt.seq, 100);
        assert_eq!(pkt.ack, 99);
        assert_eq!(pkt.payload, payload);
        assert_eq!(pkt.window_size, RECEIVE_BUFFER_BYTES);
    }

    #[test]
    fn packet_advertises_the_supplied_receive_window() {
        let packet = build_packet_ext_with_window(TYPE_STATE, 7, 4, 3, 0, 1234, &[], &[]);
        assert_eq!(parse_packet(&packet).unwrap().window_size, 1234);
    }

    #[test]
    fn packet_parser_rejects_bad_version_and_truncated_extensions() {
        let mut bad_version = build_packet(TYPE_DATA, 1, 1, 0, b"x");
        bad_version[0] = TYPE_DATA << 4;
        assert!(parse_packet(&bad_version).is_none());

        let mut truncated = build_packet(TYPE_DATA, 1, 1, 0, b"");
        truncated[1] = EXT_SACK;
        truncated.extend_from_slice(&[0, 4, 1, 2]);
        assert!(parse_packet(&truncated).is_none());
    }

    #[test]
    fn sequence_compare_handles_wraparound() {
        assert!(is_seq_before_or_equal(10, 10));
        assert!(is_seq_before_or_equal(10, 11));
        assert!(!is_seq_before_or_equal(11, 10));
        assert!(is_seq_before_or_equal(65530, 5));
        assert!(!is_seq_before_or_equal(5, 65530));
    }

    #[test]
    fn receive_flood_is_byte_bounded_and_never_acks_over_budget_data() {
        let (mut conn, _recv_rx) = receive_test_conn(100);
        let mut seq = 101u16;
        for _ in 0..(RECEIVE_BUFFER_BYTES / UTP_PAYLOAD_MAX) {
            assert_eq!(
                handle_data_packet(&mut conn, seq, &[7; UTP_PAYLOAD_MAX]),
                DataPacketOutcome::State
            );
            assert_eq!(conn.recv_seq, seq);
            seq = seq.wrapping_add(1);
        }
        let tail = RECEIVE_BUFFER_BYTES % UTP_PAYLOAD_MAX;
        assert!(tail > 0);
        assert_eq!(
            handle_data_packet(&mut conn, seq, &vec![8; tail]),
            DataPacketOutcome::State
        );
        assert_eq!(conn.recv_seq, seq);
        assert_eq!(conn.recv_budget.remaining_window(), 0);

        let rejected_seq = seq.wrapping_add(1);
        assert_eq!(
            handle_data_packet(&mut conn, rejected_seq, &[9]),
            DataPacketOutcome::State
        );
        assert_eq!(conn.recv_seq, seq, "over-budget data must not be ACKed");
        let advertised = build_state_packet(&mut conn);
        let advertised = parse_packet(&advertised).unwrap();
        assert_eq!(advertised.window_size, 0);
        assert_eq!(advertised.ack, seq);

        for _ in 1..MAX_RECEIVE_OVERFLOW_STRIKES {
            let outcome = handle_data_packet(&mut conn, rejected_seq, &[9]);
            if conn.receive_overflow_strikes < MAX_RECEIVE_OVERFLOW_STRIKES {
                assert_eq!(outcome, DataPacketOutcome::State);
            } else {
                assert_eq!(outcome, DataPacketOutcome::Reset);
            }
        }
        assert_eq!(conn.state, ConnStatus::Closed);
    }

    #[test]
    fn receive_channel_chunk_count_bounds_tiny_packet_floods() {
        let (mut conn, _recv_rx) = receive_test_conn(500);
        for offset in 1..=RECEIVE_CHANNEL_CHUNKS as u16 {
            assert_eq!(
                handle_data_packet(&mut conn, 500u16.wrapping_add(offset), &[1]),
                DataPacketOutcome::State
            );
        }
        assert_eq!(conn.recv_seq, 500 + RECEIVE_CHANNEL_CHUNKS as u16);
        assert_eq!(conn.recv_budget.remaining_window(), 0);

        let blocked = conn.recv_seq.wrapping_add(1);
        assert_eq!(
            handle_data_packet(&mut conn, blocked, &[2]),
            DataPacketOutcome::State
        );
        assert_ne!(conn.recv_seq, blocked);
        assert!(conn.ooo_received.is_empty());
        for attempt in 2..=MAX_RECEIVE_OVERFLOW_STRIKES {
            let outcome = handle_data_packet(&mut conn, blocked, &[2]);
            if attempt < MAX_RECEIVE_OVERFLOW_STRIKES {
                assert_eq!(outcome, DataPacketOutcome::State);
            } else {
                assert_eq!(outcome, DataPacketOutcome::Reset);
            }
            assert!(conn.ooo_received.is_empty());
            assert!(conn.recv_budget.bytes.load(Ordering::Acquire) <= RECEIVE_BUFFER_BYTES);
        }
        assert_eq!(conn.state, ConnStatus::Closed);
    }

    #[test]
    fn syn_limiter_bounds_global_and_per_ip_connection_growth() {
        let addr: SocketAddr = "192.0.2.1:9000".parse().unwrap();
        let mapped: SocketAddr = "[::ffff:192.0.2.1]:9001".parse().unwrap();
        let mut limiter = SynLimiter::default();
        for _ in 0..MAX_INBOUND_CONNECTIONS_PER_IP {
            assert!(limiter.allows(addr));
            limiter.opened(addr);
        }
        assert!(!limiter.allows(addr));
        assert!(!limiter.allows(mapped));
        limiter.closed(addr);
        assert!(limiter.allows(mapped));

        while limiter.total < MAX_INBOUND_CONNECTIONS {
            let index = limiter.total as u16;
            let candidate =
                SocketAddr::from(([198, 51, (index / 250) as u8, (index % 250) as u8], 1));
            if limiter.allows(candidate) {
                limiter.opened(candidate);
            }
        }
        assert!(!limiter.allows("203.0.113.1:1".parse().unwrap()));
    }

    #[test]
    fn malformed_data_payloads_reset_without_buffering() {
        let (mut conn, _recv_rx) = receive_test_conn(0);
        assert_eq!(
            handle_data_packet(&mut conn, 1, &[]),
            DataPacketOutcome::Reset
        );
        assert_eq!(conn.recv_budget.remaining_window(), RECEIVE_BUFFER_BYTES);

        let (mut conn, _recv_rx) = receive_test_conn(0);
        assert_eq!(
            handle_data_packet(&mut conn, 1, &vec![0; UTP_PAYLOAD_MAX + 1]),
            DataPacketOutcome::Reset
        );
        assert_eq!(conn.recv_budget.remaining_window(), RECEIVE_BUFFER_BYTES);
    }

    #[test]
    fn utp_stream_read_uses_channel_and_internal_buffer() {
        let (send_tx, _send_rx) = mpsc::sync_channel(SEND_CHANNEL_PACKETS);
        let (recv_tx, recv_rx) = mpsc::sync_channel(RECEIVE_CHANNEL_CHUNKS);
        let recv_budget = Arc::new(ReceiveBudget::new());
        let mut stream = UtpStream {
            addr: "127.0.0.1:1".parse().unwrap(),
            send_tx,
            recv_rx,
            recv_budget: Arc::clone(&recv_budget),
            read_buf: VecDeque::new(),
            read_timeout: Some(Duration::from_millis(200)),
            write_timeout: None,
        };

        assert!(recv_budget.try_reserve_bytes(3));
        assert!(recv_budget.try_reserve_channel_chunk());
        recv_tx.send(vec![1, 2, 3]).unwrap();
        let mut first = [0u8; 2];
        let n1 = stream.read(&mut first).unwrap();
        assert_eq!(n1, 2);
        assert_eq!(first, [1, 2]);

        let mut second = [0u8; 2];
        let n2 = stream.read(&mut second).unwrap();
        assert_eq!(n2, 1);
        assert_eq!(second[0], 3);
        assert_eq!(recv_budget.remaining_window(), RECEIVE_BUFFER_BYTES);
    }

    #[test]
    fn utp_stream_write_splits_payload_into_packets() {
        let (send_tx, send_rx) = mpsc::sync_channel(SEND_CHANNEL_PACKETS);
        let (_recv_tx, recv_rx) = mpsc::sync_channel(RECEIVE_CHANNEL_CHUNKS);
        let mut stream = UtpStream {
            addr: "127.0.0.1:1".parse().unwrap(),
            send_tx,
            recv_rx,
            recv_budget: Arc::new(ReceiveBudget::new()),
            read_buf: VecDeque::new(),
            read_timeout: None,
            write_timeout: Some(Duration::from_secs(1)),
        };

        let handle = thread::spawn(move || {
            let mut sizes = Vec::new();
            for _ in 0..2 {
                let req = send_rx.recv().unwrap();
                sizes.push(req.data.len());
                let _ = req.resp.send(Ok(()));
            }
            sizes
        });

        let total = UTP_PAYLOAD_MAX + 17;
        let written = stream.write(&vec![9u8; total]).unwrap();
        assert_eq!(written, total);
        let sizes = handle.join().unwrap();
        assert_eq!(sizes, vec![UTP_PAYLOAD_MAX, 17]);
    }

    #[test]
    fn utp_stream_write_respects_timeout() {
        let (send_tx, _send_rx) = mpsc::sync_channel::<SendRequest>(SEND_CHANNEL_PACKETS);
        let (_recv_tx, recv_rx) = mpsc::sync_channel(RECEIVE_CHANNEL_CHUNKS);
        let mut stream = UtpStream {
            addr: "127.0.0.1:1".parse().unwrap(),
            send_tx,
            recv_rx,
            recv_budget: Arc::new(ReceiveBudget::new()),
            read_buf: VecDeque::new(),
            read_timeout: None,
            write_timeout: Some(Duration::from_millis(10)),
        };

        let err = stream.write(&[1, 2, 3]).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::WouldBlock);
    }

    #[test]
    fn connector_and_listener_exchange_data() {
        fn free_port() -> u16 {
            UdpSocket::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
                .port()
        }

        let port_a = free_port();
        let port_b = free_port();
        let (connector_a, _listener_a) = start(port_a);
        let (_connector_b, listener_b) = start(port_b);

        thread::sleep(Duration::from_millis(50));
        let addr_b: SocketAddr = format!("127.0.0.1:{port_b}").parse().unwrap();
        let mut a = connector_a.connect(addr_b).unwrap();

        let deadline = Instant::now() + Duration::from_secs(3);
        let mut b = loop {
            if let Some(stream) = listener_b.try_accept() {
                break stream;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for utp accept"
            );
            thread::sleep(Duration::from_millis(10));
        };

        a.set_read_timeout(Some(Duration::from_secs(1)));
        a.set_write_timeout(Some(Duration::from_secs(1)));
        b.set_read_timeout(Some(Duration::from_secs(1)));
        b.set_write_timeout(Some(Duration::from_secs(1)));

        a.write_all(b"abc").unwrap();
        let mut recv = [0u8; 3];
        b.read_exact(&mut recv).unwrap();
        assert_eq!(&recv, b"abc");

        b.write_all(b"ok").unwrap();
        let mut recv2 = [0u8; 2];
        a.read_exact(&mut recv2).unwrap();
        assert_eq!(&recv2, b"ok");
    }

    #[test]
    fn packet_ext_preserves_timestamp_diff() {
        let pkt = build_packet_ext(TYPE_STATE, 10, 5, 4, 12345, &[], &[]);
        let parsed = parse_packet(&pkt).unwrap();
        assert_eq!(parsed.timestamp_diff, 12345);
        assert_eq!(parsed.ty, TYPE_STATE);
        assert_eq!(parsed.sack.len(), 0);
    }

    #[test]
    fn sack_extension_roundtrip() {
        let mut ooo = HashMap::new();
        // ack_nr = 10, so first missing is 11, bits represent 12, 13, 14...
        ooo.insert(12, vec![1]); // bit 0
        ooo.insert(14, vec![2]); // bit 2
        let ext = build_sack_extension(10, &ooo);
        assert!(!ext.is_empty());
        // Build a packet with that extension
        let pkt_bytes = build_packet_ext(TYPE_STATE, 1, 1, 10, 0, &ext, &[]);
        let parsed = parse_packet(&pkt_bytes).unwrap();
        assert_eq!(parsed.ty, TYPE_STATE);
        assert_eq!(parsed.ack, 10);
        assert_eq!(parsed.sack.len(), 4);
        // bit 0 and bit 2 should be set
        assert_ne!(parsed.sack[0] & 0b0000_0001, 0); // seq 12
        assert_ne!(parsed.sack[0] & 0b0000_0100, 0); // seq 14
        assert_eq!(parsed.sack[0] & 0b0000_0010, 0); // seq 13 not present
    }

    #[test]
    fn sack_extension_empty_when_no_ooo() {
        let ooo = HashMap::new();
        let ext = build_sack_extension(10, &ooo);
        assert!(ext.is_empty());
    }

    #[test]
    fn ledbat_cwnd_increases_when_delay_below_target() {
        let (_tx, rx) = mpsc::channel();
        let (dtx, _drx) = mpsc::sync_channel(RECEIVE_CHANNEL_CHUNKS);
        let recv_budget = Arc::new(ReceiveBudget::new());
        let now = Instant::now();
        let mut conn = ConnState {
            addr: "127.0.0.1:1".parse().unwrap(),
            inbound: false,
            send_id: 1,
            recv_id: Some(2),
            seq: 1,
            recv_seq: 0,
            state: ConnStatus::Connected,
            inflight: VecDeque::new(),
            inflight_bytes: 0,
            cwnd: 4,
            peer_window: RECEIVE_BUFFER_BYTES,
            send_rx: rx,
            recv_tx: dtx,
            recv_budget,
            last_advertised_window: RECEIVE_BUFFER_BYTES,
            receive_overflow_strikes: 0,
            last_seen: now,
            connect_started: now,
            connect_resp: None,
            connect_stream: None,
            peer_timestamp: 0,
            timestamp_diff: 0,
            ooo_received: HashMap::new(),
            base_delay: Some(1000), // 1ms base delay
            base_delay_updated: now,
            current_delay: 2000, // 2ms current delay (well below 100ms target)
        };
        let old_cwnd = conn.cwnd;
        ledbat_update_cwnd(&mut conn, 4 * UTP_PAYLOAD_MAX);
        // With very low queuing delay the cwnd should not decrease
        assert!(conn.cwnd >= old_cwnd);
    }

    #[test]
    fn ledbat_cwnd_decreases_when_delay_above_target() {
        let (_tx, rx) = mpsc::channel();
        let (dtx, _drx) = mpsc::sync_channel(RECEIVE_CHANNEL_CHUNKS);
        let recv_budget = Arc::new(ReceiveBudget::new());
        let now = Instant::now();
        let mut conn = ConnState {
            addr: "127.0.0.1:1".parse().unwrap(),
            inbound: false,
            send_id: 1,
            recv_id: Some(2),
            seq: 1,
            recv_seq: 0,
            state: ConnStatus::Connected,
            inflight: VecDeque::new(),
            inflight_bytes: 0,
            cwnd: 20,
            peer_window: RECEIVE_BUFFER_BYTES,
            send_rx: rx,
            recv_tx: dtx,
            recv_budget,
            last_advertised_window: RECEIVE_BUFFER_BYTES,
            receive_overflow_strikes: 0,
            last_seen: now,
            connect_started: now,
            connect_resp: None,
            connect_stream: None,
            peer_timestamp: 0,
            timestamp_diff: 0,
            ooo_received: HashMap::new(),
            base_delay: Some(1000),
            base_delay_updated: now,
            current_delay: 500_000, // 500ms current delay >> 100ms target
        };
        let old_cwnd = conn.cwnd;
        ledbat_update_cwnd(&mut conn, 20 * UTP_PAYLOAD_MAX);
        // With high queuing delay the cwnd should decrease
        assert!(conn.cwnd < old_cwnd);
    }
}
