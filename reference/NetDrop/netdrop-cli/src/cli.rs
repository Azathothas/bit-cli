// ============================================================
//  netdrop // cli.rs — терминальный UX: прогрессбары (indicatif),
//  Y/n-подтверждение (dialoguer), QR-тикет, вывод тикетов.
//  Все пользовательские сообщения — на английском.
// ============================================================

use indicatif::{ProgressBar, ProgressStyle};
use netdrop_core::crypto::Ticket;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Шапка утилиты.
pub fn banner() {
    println!("netdrop {} — zero-knowledge P2P file transfer\n", env!("CARGO_PKG_VERSION"));
}

/// Крутилка для ожиданий (сеть, подтверждения).
pub fn spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .expect("spinner template")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "·"]),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(120));
    pb
}

/// Прогрессбар передачи: байты, скорость, ETA.
pub fn progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            "  {bar:34.cyan/blue} {bytes}/{total_bytes} · {bytes_per_sec} · ETA {eta} {msg}",
        )
        .expect("bar template")
        .progress_chars("━╸─"),
    );
    pb
}

/// Фоновая задача: раз в 100 мс переносит атомарный счётчик в прогрессбар.
pub fn watch_progress(bar: ProgressBar, counter: Arc<AtomicU64>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            bar.set_position(counter.load(Ordering::Relaxed));
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    })
}

/// Y/n-подтверждение (dialoguer блокирует — уводим в spawn_blocking).
pub async fn confirm(prompt: String) -> anyhow::Result<bool> {
    tokio::task::spawn_blocking(move || {
        dialoguer::Confirm::new()
            .with_prompt(prompt)
            .default(true)
            .interact()
            .map_err(|e| anyhow::anyhow!("confirmation prompt: {e}"))
    })
    .await?
}

/// QR-код тикета в терминале (инверсия под тёмный фон, EC-уровень L).
pub fn print_qr(data: &str) {
    use qrcode::render::unicode;
    match qrcode::QrCode::with_error_correction_level(data.as_bytes(), qrcode::EcLevel::L) {
        Ok(code) => {
            let art = code
                .render::<unicode::Dense1x2>()
                .dark_color(unicode::Dense1x2::Light)
                .light_color(unicode::Dense1x2::Dark)
                .quiet_zone(true)
                .build();
            for line in art.lines() {
                println!("  {line}");
            }
            println!();
        }
        Err(e) => println!("  (QR code unavailable: {e})\n"),
    }
}

/// Человекочитаемый размер (1024-кратный).
pub fn human_bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = n as f64;
    let mut u = 0usize;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{n} B")
    } else {
        format!("{v:.2} {}", UNITS[u])
    }
}

/// Безопасное имя файла: без разделителей пути и запрещённых символов.
pub fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if (c as u32) < 0x20 => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').to_string();
    if trimmed.is_empty() {
        "netdrop.bin".to_string()
    } else {
        trimmed
    }
}

/// Свободный путь в каталоге: name.ext → name (1).ext → name (2).ext …
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let safe = sanitize_name(name);
    let candidate = dir.join(&safe);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = match safe.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (safe.clone(), String::new()),
    };
    for i in 1..10_000u32 {
        let p = dir.join(format!("{stem} ({i}){ext}"));
        if !p.exists() {
            return p;
        }
    }
    dir.join(format!("{stem} ({}){ext}", std::process::id()))
}

/// Печать содержимого тикета (inspect).
pub fn print_ticket(t: &Ticket) {
    println!("  ticket OK (nd1, checksum verified)");
    println!("  e2e encryption: {}", if t.encrypted { "ON" } else { "OFF" });
    println!("  session pubkey: {}", hex(&t.session_pubkey));
    println!("  node id:        {}", hex(&t.node_id));
    match &t.relay_url {
        Some(r) => println!("  relay:          {r}"),
        None => println!("  relay:          — (direct / hole punching only)"),
    }
    if t.direct_addrs.is_empty() {
        println!("  direct addrs:   —");
    } else {
        for a in &t.direct_addrs {
            println!("  direct addr:    {a}");
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_bytes_formats() {
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(2048), "2.00 KiB");
        assert_eq!(human_bytes(64 * 1024), "64.00 KiB");
        assert_eq!(human_bytes(5 * 1024 * 1024), "5.00 MiB");
    }

    #[test]
    fn sanitize_strips_bad_chars() {
        assert_eq!(sanitize_name("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(sanitize_name("a:b*c?.txt"), "a_b_c_.txt");
        assert_eq!(sanitize_name(""), "netdrop.bin");
    }
}
