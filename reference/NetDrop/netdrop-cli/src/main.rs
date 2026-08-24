// ============================================================
//  netdrop // main.rs — CLI: send / receive / inspect.
//  Итерация 3: папки (tar-поток), докачка (.ndpart + Preseed),
//  QR-тикет в терминале, динамический chunk_size.
//  Все пользовательские сообщения — на английском.
// ============================================================

mod cli;

use anyhow::{anyhow, bail, Context};
use clap::{Parser, Subcommand};
use netdrop_core::crypto::{EphemeralKeyPair, Role, Ticket};
use netdrop_core::network::protocol as proto;
use netdrop_core::network::protocol::FrameType;
use netdrop_core::network::transport::Transport;
use netdrop_core::pipeline::{self, open_frame, seal_frame, PipelineError, Preseed};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio_util::io::SyncIoBridge;

#[derive(Parser)]
#[command(name = "netdrop", version, about = "Zero-knowledge P2P file transfer (E2EE, PFS)")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Send a file or folder (generates a one-shot nd1… ticket)
    Send {
        /// Path to a file or folder
        path: PathBuf,
        /// Disable the application E2E layer (QUIC transport stays encrypted)
        #[arg(long)]
        no_encrypt: bool,
        /// Disable ZSTD stream compression
        #[arg(long)]
        no_compress: bool,
        /// Extra relay URL (added to n0 + SmartHoldem n1 relays)
        #[arg(long)]
        relay: Option<String>,
        /// Do not print the QR code for the ticket
        #[arg(long)]
        no_qr: bool,
        /// Upload speed limit in KiB/s (0 = unlimited)
        #[arg(long, default_value_t = 0)]
        limit: u64,
    },
    /// Receive a transfer using an nd1… ticket
    Receive {
        /// Ticket from the sender (nd1…)
        ticket: String,
        /// Destination directory (default: current)
        #[arg(short, long)]
        out: Option<PathBuf>,
        /// Skip the Y/n confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },
    /// Decode a ticket and print its contents (diagnostics)
    Inspect {
        /// nd1… ticket
        ticket: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    cli::banner();
    tokio::select! {
        res = run(args.command) => res,
        _ = tokio::signal::ctrl_c() => {
            println!("\n⏹ interrupted (Ctrl+C) — connection closed, session key destroyed");
            println!("  partial downloads are kept as *.ndpart and will resume with a new ticket");
            Ok(())
        }
    }
}

async fn run(cmd: Commands) -> anyhow::Result<()> {
    match cmd {
        Commands::Send { path, no_encrypt, no_compress, relay, no_qr, limit } => {
            send(path, !no_encrypt, !no_compress, relay, !no_qr, limit).await
        }
        Commands::Receive { ticket, out, yes } => receive(ticket, out, yes).await,
        Commands::Inspect { ticket } => {
            let t = Ticket::decode(&ticket).map_err(|e| anyhow!("ticket: {e}"))?;
            cli::print_ticket(&t);
            Ok(())
        }
    }
}

/// Рекурсивная статистика папки: (суммарный размер, число файлов).
async fn dir_stats(path: &Path) -> anyhow::Result<(u64, u32)> {
    let root = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        fn walk(dir: &Path, total: &mut u64, count: &mut u32) -> std::io::Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let meta = entry.metadata()?;
                if meta.is_dir() {
                    walk(&entry.path(), total, count)?;
                } else if meta.is_file() {
                    *total += meta.len();
                    *count += 1;
                }
            }
            Ok(())
        }
        let mut total = 0u64;
        let mut count = 0u32;
        walk(&root, &mut total, &mut count)?;
        Ok::<(u64, u32), std::io::Error>((total, count))
    })
    .await?
    .map_err(|e| anyhow!("folder scan failed: {e}"))
}

// ---------- Отправитель ----------

async fn send(
    path: PathBuf,
    encrypted: bool,
    compress: bool,
    relay: Option<String>,
    show_qr: bool,
    limit_kib: u64,
) -> anyhow::Result<()> {
    let meta_fs = tokio::fs::metadata(&path)
        .await
        .with_context(|| format!("path not found: {}", path.display()))?;
    let is_dir = meta_fs.is_dir();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "netdrop.bin".to_string());
    let (size, file_count) = if is_dir {
        dir_stats(&path).await?
    } else {
        (meta_fs.len(), 1)
    };
    let chunk = proto::optimal_chunk_size(size);

    let rate_limit = (limit_kib > 0).then(|| limit_kib * 1024);
    let kp = encrypted.then(EphemeralKeyPair::generate);

    let spin = cli::spinner("joining the P2P network (n0+n1 relays, discovery)…");
    let extra: Vec<String> = relay.into_iter().collect();
    let transport = Transport::bind(&extra, true).await?;
    let (node_id, relay_url, direct_addrs) = transport.ticket_parts().await;
    spin.finish_and_clear();

    let ticket = Ticket {
        encrypted,
        session_pubkey: kp.as_ref().map(|k| k.public_bytes()).unwrap_or([0u8; 32]),
        node_id,
        relay_url,
        direct_addrs,
    };
    println!(
        "→ {}: {} ({}{}) · e2e: {} · zstd: {} · chunk: {}",
        if is_dir { "folder" } else { "file" },
        name,
        cli::human_bytes(size),
        if is_dir { format!(", {file_count} files") } else { String::new() },
        on_off(encrypted),
        on_off(compress),
        cli::human_bytes(chunk as u64),
    );
    println!();
    println!("  TICKET (give it to the receiver, single-use):");
    println!("  {}", ticket.encode());
    println!();
    if show_qr {
        cli::print_qr(&ticket.encode());
    }

    let spin = cli::spinner("waiting for the receiver…");
    let conn = transport.accept_one().await?;
    spin.finish_and_clear();
    println!("✓ receiver connected: {}", conn.remote_id().fmt_short());

    let (mut tx, mut rx) = conn
        .accept_bi()
        .await
        .map_err(|e| anyhow!("bi-stream failed to open: {e}"))?;

    // 1) Handshake от получателя (plaintext).
    let hs_frame = proto::read_frame(&mut rx).await?;
    if hs_frame.frame_type != FrameType::Handshake {
        bail!("expected Handshake, got frame 0x{:02x}", hs_frame.frame_type.as_byte());
    }
    let hs: proto::HandshakePayload = proto::decode_payload(&hs_frame.payload)?;
    if hs.protocol_version != proto::PROTOCOL_VERSION {
        let err = proto::ErrorPayload {
            code: proto::ERR_VERSION,
            message: format!("protocol version mismatch: expected {}", proto::PROTOCOL_VERSION),
        };
        let f = proto::Frame::plain(FrameType::Error, proto::encode_payload(&err)?);
        proto::write_frame(&mut tx, &f).await?;
        bail!("incompatible receiver protocol version: {}", hs.protocol_version);
    }

    // 2) Ключи сессии (PFS) + отпечаток.
    let session = match kp {
        Some(k) => Some(k.derive_session(&hs.receiver_pubkey, Role::Sender)?),
        None => None,
    };
    let (mut sealer, mut opener) = match &session {
        Some(s) => (Some(s.sealer()), Some(s.opener())),
        None => (None, None),
    };
    if let Some(s) = &session {
        println!("🔐 session fingerprint: {}  (verify it with the receiver)", s.fingerprint());
    }

    // 3) MetaInfo → ждём подтверждение (Ack несёт resume-offset получателя).
    let meta = proto::MetaInfo {
        name: name.clone(),
        size,
        is_dir,
        file_count,
        compressed: compress,
        encrypted,
        chunk_size: chunk as u32,
    };
    let mf = seal_frame(&mut sealer, FrameType::MetaInfo, &proto::encode_payload(&meta)?)?;
    proto::write_frame(&mut tx, &mf).await?;

    let spin = cli::spinner("waiting for receiver confirmation (Y/n)…");
    let resp = proto::read_frame(&mut rx).await?;
    spin.finish_and_clear();
    let offset = match resp.frame_type {
        FrameType::Ack => {
            let ack: proto::AckPayload = proto::decode_payload(&open_frame(&mut opener, &resp)?)?;
            // докачка только для одиночных файлов (tar-поток недетерминирован)
            if !is_dir && ack.received_bytes > 0 && ack.received_bytes < size {
                ack.received_bytes
            } else {
                0
            }
        }
        FrameType::Error => {
            let e: proto::ErrorPayload = proto::decode_payload(&resp.payload)?;
            bail!("receiver rejected the transfer: {}", e.message);
        }
        other => bail!("expected Ack/Error, got frame 0x{:02x}", other.as_byte()),
    };
    if offset > 0 {
        println!("↻ resuming from {} (receiver already has a partial copy)", cli::human_bytes(offset));
    }

    // 4) Стрим данных с прогрессбаром.
    let progress = Arc::new(AtomicU64::new(0));
    // для папки tar добавляет заголовки — оценка длины бара с запасом
    let bar_len = if is_dir { size + (file_count as u64) * 1024 + 4096 } else { size };
    let bar = cli::progress_bar(bar_len);
    let watcher = cli::watch_progress(bar.clone(), progress.clone());

    let outcome = if is_dir {
        // папка → tar-поток (структура сохраняется) через duplex-мост
        let (tar_read, tar_write) = tokio::io::duplex(1 << 20);
        let src_path = path.clone();
        let root_name = name.clone();
        let tar_task = tokio::task::spawn_blocking(move || -> std::io::Result<()> {
            let bridge = SyncIoBridge::new(tar_write);
            let mut builder = tar::Builder::new(bridge);
            builder.append_dir_all(&root_name, &src_path)?;
            let mut bridge = builder.into_inner()?;
            std::io::Write::flush(&mut bridge)?;
            Ok(())
        });
        let res = pipeline::send_stream(
            tar_read, &mut tx, chunk, compress, &mut sealer, progress, None, rate_limit,
        )
        .await;
        let tar_res = tar_task.await;
        let outcome = res?;
        tar_res
            .map_err(|e| anyhow!("tar task panicked: {e}"))?
            .map_err(|e| anyhow!("folder packing failed: {e}"))?;
        outcome
    } else {
        let mut file = tokio::fs::File::open(&path).await?;
        let preseed = if offset > 0 {
            // префикс хэшируется, курсор файла остаётся на offset
            Some(Preseed::from_reader(&mut file, offset).await?)
        } else {
            None
        };
        pipeline::send_stream(file, &mut tx, chunk, compress, &mut sealer, progress, preseed, rate_limit)
            .await?
    };
    watcher.abort();
    bar.set_position(bar_len);
    bar.finish_with_message("sent");
    tx.finish().map_err(|e| anyhow!("stream finish: {e}"))?;

    // 5) Финальное подтверждение целостности от получателя.
    let fin = proto::read_frame(&mut rx).await?;
    match fin.frame_type {
        FrameType::Ack => {
            let _ = open_frame(&mut opener, &fin)?;
            println!(
                "✓ done: {} ({}) delivered — receiver's SHA-256 matches",
                name,
                cli::human_bytes(outcome.bytes)
            );
        }
        FrameType::Error => {
            let e: proto::ErrorPayload = proto::decode_payload(&fin.payload)?;
            bail!("receiver reported an error: {}", e.message);
        }
        other => bail!("expected final Ack, got frame 0x{:02x}", other.as_byte()),
    }

    conn.close(0u32.into(), b"done");
    transport.close().await;
    Ok(())
}

// ---------- Получатель ----------

async fn receive(ticket: String, out: Option<PathBuf>, yes: bool) -> anyhow::Result<()> {
    let t = Ticket::decode(&ticket).map_err(|e| anyhow!("ticket: {e}"))?;
    let kp = t.encrypted.then(EphemeralKeyPair::generate);

    let spin = cli::spinner("connecting to the sender (hole punching / relays)…");
    let transport = Transport::bind(&[], true).await?;
    let conn = transport.connect(&t).await?;
    spin.finish_and_clear();
    println!("✓ connected: {}", conn.remote_id().fmt_short());

    let (mut tx, mut rx) = conn
        .open_bi()
        .await
        .map_err(|e| anyhow!("bi-stream failed to open: {e}"))?;

    // 1) Handshake: наш эфемерный ключ (plaintext).
    let hs = proto::HandshakePayload {
        protocol_version: proto::PROTOCOL_VERSION,
        receiver_pubkey: kp.as_ref().map(|k| k.public_bytes()).unwrap_or([0u8; 32]),
    };
    let f = proto::Frame::plain(FrameType::Handshake, proto::encode_payload(&hs)?);
    proto::write_frame(&mut tx, &f).await?;

    // 2) Ключи сессии + отпечаток.
    let session = match kp {
        Some(k) => Some(k.derive_session(&t.session_pubkey, Role::Receiver)?),
        None => None,
    };
    let (mut sealer, mut opener) = match &session {
        Some(s) => (Some(s.sealer()), Some(s.opener())),
        None => (None, None),
    };
    if let Some(s) = &session {
        println!("🔐 session fingerprint: {}  (verify it with the sender)", s.fingerprint());
    }

    // 3) MetaInfo → подтверждение Y/n.
    let mf = proto::read_frame(&mut rx).await?;
    let meta: proto::MetaInfo = match mf.frame_type {
        FrameType::MetaInfo => proto::decode_payload(&open_frame(&mut opener, &mf)?)?,
        FrameType::Error => {
            let e: proto::ErrorPayload = proto::decode_payload(&mf.payload)?;
            bail!("sender reported an error: {}", e.message);
        }
        other => bail!("expected MetaInfo, got frame 0x{:02x}", other.as_byte()),
    };
    let chunk = (meta.chunk_size as usize).min(proto::MAX_CHUNK_SIZE);
    println!(
        "📦 incoming {}: {} ({}{}) · e2e: {} · zstd: {} · chunk: {}",
        if meta.is_dir { "folder" } else { "file" },
        meta.name,
        cli::human_bytes(meta.size),
        if meta.is_dir { format!(", {} files", meta.file_count) } else { String::new() },
        on_off(meta.encrypted),
        on_off(meta.compressed),
        cli::human_bytes(chunk as u64),
    );
    let accept = yes || cli::confirm(format!("Accept \"{}\"?", meta.name)).await?;
    if !accept {
        let err = proto::ErrorPayload {
            code: proto::ERR_REJECTED,
            message: "rejected by receiver".to_string(),
        };
        let f = proto::Frame::plain(FrameType::Error, proto::encode_payload(&err)?);
        proto::write_frame(&mut tx, &f).await?;
        conn.close(0u32.into(), b"rejected");
        transport.close().await;
        println!("transfer rejected");
        return Ok(());
    }

    let out_dir = out.unwrap_or_else(|| PathBuf::from("."));
    tokio::fs::create_dir_all(&out_dir).await.ok();

    // Докачка (только файлы): частичная копия *.ndpart + сайдкар-метаданные.
    let safe_name = cli::sanitize_name(&meta.name);
    let part_path = out_dir.join(format!("{safe_name}.ndpart"));
    let sidecar_path = out_dir.join(format!("{safe_name}.ndpart.json"));
    let mut resume_offset = 0u64;
    let mut preseed: Option<Preseed> = None;
    if !meta.is_dir {
        if let (Ok(part_meta), Ok(raw)) = (
            tokio::fs::metadata(&part_path).await,
            tokio::fs::read_to_string(&sidecar_path).await,
        ) {
            let same = serde_json::from_str::<serde_json::Value>(&raw)
                .ok()
                .map(|v| {
                    v["name"].as_str() == Some(meta.name.as_str())
                        && v["size"].as_u64() == Some(meta.size)
                })
                .unwrap_or(false);
            let plen = part_meta.len();
            if same && plen > 0 && plen < meta.size {
                let mut pf = tokio::fs::File::open(&part_path).await?;
                preseed = Some(Preseed::from_reader(&mut pf, plen).await?);
                resume_offset = plen;
                println!("↻ resuming from {} (partial copy found)", cli::human_bytes(plen));
            }
        }
    }

    // Ack(accept) несёт resume-offset для отправителя.
    let ack = proto::AckPayload { received_bytes: resume_offset };
    let f = seal_frame(&mut sealer, FrameType::Ack, &proto::encode_payload(&ack)?)?;
    proto::write_frame(&mut tx, &f).await?;

    // 4) Приём с прогрессбаром.
    let progress = Arc::new(AtomicU64::new(0));
    let bar = cli::progress_bar(meta.size.max(1));
    let watcher = cli::watch_progress(bar.clone(), progress.clone());

    let (res, final_display): (Result<pipeline::TransferOutcome, PipelineError>, PathBuf) =
        if meta.is_dir {
            // tar-поток → распаковка прямо в каталог (структура сохраняется)
            let (untar_read, mut untar_write) = tokio::io::duplex(1 << 20);
            let od = out_dir.clone();
            let untar_task = tokio::task::spawn_blocking(move || -> std::io::Result<()> {
                let bridge = SyncIoBridge::new(untar_read);
                let mut archive = tar::Archive::new(bridge);
                archive.unpack(&od)
            });
            let res = pipeline::receive_stream(
                &mut rx, &mut untar_write, chunk, meta.compressed, &mut opener,
                progress.clone(), None,
            )
            .await;
            let _ = untar_write.shutdown().await;
            let tar_res = untar_task.await;
            let res = match (res, tar_res) {
                (Ok(o), Ok(Ok(()))) => Ok(o),
                (Ok(_), Ok(Err(e))) => Err(PipelineError::Io(e)),
                (Ok(_), Err(e)) => Err(PipelineError::Io(std::io::Error::other(e))),
                (Err(e), _) => Err(e),
            };
            (res, out_dir.join(&safe_name))
        } else {
            // сайдкар пишем ДО приёма — обрыв оставит валидную точку докачки
            let sidecar = serde_json::json!({ "name": meta.name, "size": meta.size });
            tokio::fs::write(&sidecar_path, sidecar.to_string()).await.ok();
            let mut opts = tokio::fs::OpenOptions::new();
            opts.create(true).write(true);
            if resume_offset > 0 {
                opts.append(true);
            } else {
                opts.truncate(true);
            }
            let mut file = opts
                .open(&part_path)
                .await
                .with_context(|| format!("cannot open {}", part_path.display()))?;
            let res = pipeline::receive_stream(
                &mut rx, &mut file, chunk, meta.compressed, &mut opener,
                progress.clone(), preseed,
            )
            .await;
            (res, part_path.clone())
        };
    watcher.abort();

    match res {
        Ok(outcome) => {
            bar.set_position(meta.size);
            bar.finish_with_message("received");
            let ack = proto::AckPayload { received_bytes: outcome.bytes };
            let f = seal_frame(&mut sealer, FrameType::Ack, &proto::encode_payload(&ack)?)?;
            proto::write_frame(&mut tx, &f).await?;
            let _ = tx.finish();
            let shown = if meta.is_dir {
                final_display
            } else {
                // *.ndpart → финальное имя (с авто-переименованием при коллизии)
                let dest = cli::unique_path(&out_dir, &meta.name);
                tokio::fs::rename(&part_path, &dest).await?;
                let _ = tokio::fs::remove_file(&sidecar_path).await;
                dest
            };
            println!(
                "✓ saved: {} ({}) — SHA-256 verified",
                shown.display(),
                cli::human_bytes(outcome.bytes)
            );
            conn.close(0u32.into(), b"done");
            transport.close().await;
            Ok(())
        }
        Err(e) => {
            bar.abandon_with_message("failed");
            let code = if matches!(e, PipelineError::HashMismatch) {
                proto::ERR_HASH_MISMATCH
            } else {
                proto::ERR_INTERNAL
            };
            let err = proto::ErrorPayload { code, message: e.to_string() };
            if let Ok(payload) = proto::encode_payload(&err) {
                let f = proto::Frame::plain(FrameType::Error, payload);
                let _ = proto::write_frame(&mut tx, &f).await;
            }
            if meta.is_dir || matches!(e, PipelineError::HashMismatch) {
                // хэш не сошёлся — частичная копия бесполезна
                if !meta.is_dir {
                    let _ = tokio::fs::remove_file(&part_path).await;
                    let _ = tokio::fs::remove_file(&sidecar_path).await;
                }
            } else {
                println!(
                    "  partial copy kept: {} — run `netdrop receive` with a new ticket to resume",
                    part_path.display()
                );
            }
            conn.close(1u32.into(), b"error");
            transport.close().await;
            Err(e.into())
        }
    }
}

fn on_off(v: bool) -> &'static str {
    if v { "ON" } else { "OFF" }
}
