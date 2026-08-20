//! Probing an HTTP source without downloading from it.
//!
//! Two questions, two functions.
//!
//! [`test_source`] answers "can this mirror serve this torrent at all": does it
//! answer, does it honour `Range`, is the entity the size the torrent says,
//! where does it end up after redirects, and what TLS did it negotiate. One
//! request per source, one byte of payload at most.
//!
//! [`probe_source`] answers "how well": ranged-GET latency percentiles and how
//! throughput moves as concurrency rises. It reads real payload and throws it
//! away, so it costs bandwidth and is not something to run by accident.
//!
//! Both are read-only. Neither writes a payload and neither touches the
//! `.torrent`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use hdrhistogram::Histogram;
use reqwest::header::{ACCEPT_RANGES, CONTENT_LENGTH, CONTENT_RANGE, LOCATION, RANGE, SERVER};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::layout::Layout;
use crate::time::Timestamp;
use crate::units::{Size, format_rate, format_size};
use crate::webseed::binding::{Auth, Binding};
use crate::webseed::fetch::default_user_agent;

/// Redirect hops followed before giving up.
///
/// A chain longer than this is a misconfiguration rather than a route, and
/// following it forever turns one probe into an outage.
const MAX_REDIRECTS: usize = 10;

/// Whether a source honours `Range`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RangeSupport {
    /// The server answered `206` with a matching `Content-Range`.
    Yes,
    /// The server answered `200` and sent the whole entity.
    No,
    /// The request failed, so nothing was learned either way.
    Unknown,
}

impl RangeSupport {
    /// The stable name used in output.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Yes => "yes",
            Self::No => "no",
            Self::Unknown => "unknown",
        }
    }
}

/// What TLS was negotiated, when the source is HTTPS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsReport {
    /// `TLSv1.3` or `TLSv1.2`.
    pub version: String,
    /// The negotiated cipher suite, by its IANA name.
    pub cipher_suite: String,
    /// The name sent in SNI, which is the host the certificate is checked
    /// against.
    pub server_name: String,
    /// The protocol agreed by ALPN, when there was one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpn: Option<String>,
    /// Time to open the TCP connection.
    pub connect_ms: u64,
    /// Time to complete the TLS handshake, on top of the connection.
    pub handshake_ms: u64,
}

/// One redirect hop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hop {
    pub status: u16,
    pub from: String,
    pub to: String,
}

/// What `bit-cli webseed test` reports for one source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceTest {
    pub index: usize,
    /// The source as declared.
    pub url: String,
    /// The URL the probe actually requested, after composition.
    pub request_url: String,
    pub origin: String,
    pub scope: String,
    pub mode: String,
    /// Whether the source can serve this torrent.
    pub ok: bool,
    /// `HEAD` or `GET`.
    pub method: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    /// Where the request ended up after redirects, when that differs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_url: Option<String>,
    /// Every redirect hop, in order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub redirects: Vec<Hop>,
    pub range_support: RangeSupport,
    /// Length the server reports for the whole entity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_length: Option<u64>,
    /// Length the torrent says the file has.
    pub expected_length: u64,
    /// Whether the two agree. `None` when the server did not say.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length_matches: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    /// `HTTP/1.1`, `HTTP/2.0`, and so on.
    pub http_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsReport>,
    /// Time to the first byte of the response.
    pub ttfb_ms: u64,
    /// Time for the whole exchange, redirects included.
    pub total_ms: u64,
    pub at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl SourceTest {
    /// A source that was never reached, because the command ran out of time
    /// before its turn came.
    ///
    /// Reporting it rather than dropping it is what keeps the source count in
    /// the report equal to the number of sources the torrent declares.
    pub fn unfinished(binding: &Binding, reason: &str) -> Self {
        Self::failed(
            binding,
            binding.spec.url.clone(),
            "GET",
            Duration::ZERO,
            reason.to_string(),
        )
    }

    fn failed(
        binding: &Binding,
        request_url: String,
        method: &'static str,
        elapsed: Duration,
        reason: String,
    ) -> Self {
        Self {
            index: binding.index,
            url: binding.spec.url.clone(),
            request_url,
            origin: binding.spec.origin.as_str().to_string(),
            scope: binding.scope.selector.clone(),
            mode: binding.spec.mode.as_str().to_string(),
            ok: false,
            method,
            status: None,
            resolved_url: None,
            redirects: Vec::new(),
            range_support: RangeSupport::Unknown,
            content_length: None,
            expected_length: 0,
            length_matches: None,
            server: None,
            http_version: String::new(),
            tls: None,
            ttfb_ms: 0,
            total_ms: elapsed.as_millis().min(u128::from(u64::MAX)) as u64,
            at: Timestamp::now().iso(),
            error: Some(reason),
        }
    }
}

/// Probe one source: does it answer, does it do ranges, is it the right size.
///
/// Redirects are followed by hand rather than by the HTTP client, so the chain
/// is reported hop by hop. A mirror that quietly redirects to a login page is
/// otherwise indistinguishable from one that works.
pub async fn test_source(
    binding: &Binding,
    layout: &Layout,
    info_hash: &str,
    use_head: bool,
) -> SourceTest {
    let started = Instant::now();
    let at = Timestamp::now();
    let method = match use_head {
        true => "HEAD",
        false => "GET",
    };

    // Probe the first in-scope file, which is the one a download would ask
    // for first.
    let Some(&file) = binding.scope.files.first() else {
        return SourceTest::failed(
            binding,
            binding.spec.url.clone(),
            method,
            started.elapsed(),
            "the scope selects no file, so there is nothing to probe".to_string(),
        );
    };
    let Some(entry) = layout.file(file) else {
        return SourceTest::failed(
            binding,
            binding.spec.url.clone(),
            method,
            started.elapsed(),
            format!("file index {file} is not in the torrent"),
        );
    };
    let request_url = match binding.url_for(layout, info_hash, entry.offset, entry.length.max(1)) {
        Ok(url) => url,
        Err(e) => {
            return SourceTest::failed(
                binding,
                binding.spec.url.clone(),
                method,
                started.elapsed(),
                e.to_string(),
            );
        }
    };

    // A local source answers from the filesystem, so there is no request to
    // make, no redirect to follow, and no TLS to report. What a caller wants
    // to know is the same though: is it there, and is it the right size.
    if crate::webseed::local::is_file_url(&request_url) {
        return test_local(binding, &request_url, entry.length, started);
    }

    let client = match probe_client(binding) {
        Ok(client) => client,
        Err(e) => {
            return SourceTest::failed(
                binding,
                request_url,
                method,
                started.elapsed(),
                e.to_string(),
            );
        }
    };

    let mut current = request_url.clone();
    let mut redirects = Vec::new();
    let mut ttfb;

    let response = loop {
        let began = Instant::now();
        let mut request = client.request(
            match use_head {
                true => reqwest::Method::HEAD,
                false => reqwest::Method::GET,
            },
            &current,
        );
        request = request.headers(headers(binding));
        if !use_head {
            // One byte is enough to learn whether the server does ranges, and
            // it costs the mirror nothing.
            request = request.header(RANGE, "bytes=0-0");
        }
        if let Auth::Basic { user, password } = &binding.spec.auth {
            request = request.basic_auth(user, Some(password));
        }

        let sent = match request.send().await {
            Ok(response) => response,
            Err(e) => {
                return SourceTest::failed(
                    binding,
                    request_url,
                    method,
                    started.elapsed(),
                    format!("{current}: {e}"),
                );
            }
        };
        ttfb = began.elapsed();

        let status = sent.status();
        if !status.is_redirection() {
            break sent;
        }
        let Some(location) = sent.headers().get(LOCATION).and_then(|v| v.to_str().ok()) else {
            return SourceTest::failed(
                binding,
                request_url,
                method,
                started.elapsed(),
                format!("{current}: {status} with no Location header"),
            );
        };
        let next = match resolve_redirect(&current, location) {
            Ok(next) => next,
            Err(e) => {
                return SourceTest::failed(
                    binding,
                    request_url,
                    method,
                    started.elapsed(),
                    e.to_string(),
                );
            }
        };
        redirects.push(Hop {
            status: status.as_u16(),
            from: current.clone(),
            to: next.clone(),
        });
        if redirects.len() > MAX_REDIRECTS {
            let reason = format!("more than {MAX_REDIRECTS} redirects starting at {request_url}");
            return SourceTest::failed(binding, request_url, method, started.elapsed(), reason);
        }
        current = next;
    };

    let status = response.status();
    let headers = response.headers().clone();
    let http_version = format!("{:?}", response.version());
    let server = headers
        .get(SERVER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    // A `Content-Range` gives the true entity size; a `Content-Length` on a
    // 206 is the size of the one byte that came back, not of the file.
    let content_length = match status.as_u16() {
        206 => headers
            .get(CONTENT_RANGE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| {
                v.rsplit('/')
                    .next()
                    .and_then(|total| total.parse::<u64>().ok())
            }),
        _ => headers
            .get(CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok()),
    };

    let range_support = match (use_head, status.as_u16()) {
        (_, 206) => RangeSupport::Yes,
        (true, 200) => match headers.get(ACCEPT_RANGES).and_then(|v| v.to_str().ok()) {
            Some(value) if value.eq_ignore_ascii_case("bytes") => RangeSupport::Yes,
            Some(_) => RangeSupport::No,
            None => RangeSupport::Unknown,
        },
        (false, 200) => RangeSupport::No,
        _ => RangeSupport::Unknown,
    };

    // The TLS probe gets the source's own connect timeout rather than the
    // default, because a caller who said "give up on this mirror after 2s"
    // meant it for every connection the probe opens, not only the one
    // `reqwest` makes.
    let tls = match current.to_ascii_lowercase().starts_with("https://") {
        true => tls_report_within(&current, binding.spec.limits.connect_timeout())
            .await
            .ok(),
        false => None,
    };

    let length_matches = content_length.map(|len| len == entry.length);
    let ok =
        status.is_success() && length_matches != Some(false) && range_support != RangeSupport::No;
    let error = match (status.is_success(), length_matches, range_support) {
        (false, _, _) => Some(format!("HTTP {status}")),
        (_, Some(false), _) => Some(format!(
            "the server says {} bytes but the torrent says {}",
            content_length.unwrap_or(0),
            entry.length
        )),
        (_, _, RangeSupport::No) => Some("the server does not honour Range".to_string()),
        _ => None,
    };

    SourceTest {
        index: binding.index,
        url: binding.spec.url.clone(),
        request_url: request_url.clone(),
        origin: binding.spec.origin.as_str().to_string(),
        scope: binding.scope.selector.clone(),
        mode: binding.spec.mode.as_str().to_string(),
        ok,
        method,
        status: Some(status.as_u16()),
        resolved_url: (current != request_url).then_some(current),
        redirects,
        range_support,
        content_length,
        expected_length: entry.length,
        length_matches,
        server,
        http_version,
        tls,
        ttfb_ms: ttfb.as_millis().min(u128::from(u64::MAX)) as u64,
        total_ms: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        at: at.iso(),
        error,
    }
}

/// One step of a concurrency sweep.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcurrencyStep {
    pub concurrency: usize,
    pub requests: u64,
    pub errors: u64,
    pub bytes: u64,
    pub bytes_human: String,
    pub elapsed_ms: u64,
    /// Bytes per second sustained at this concurrency.
    pub throughput: u64,
    pub throughput_human: String,
    /// Request-to-completion latency, in milliseconds.
    pub p50_ms: u64,
    pub p90_ms: u64,
    pub p99_ms: u64,
    pub p999_ms: u64,
    pub max_ms: u64,
    /// Request-to-first-byte latency, in milliseconds.
    pub ttfb_p50_ms: u64,
    pub ttfb_p99_ms: u64,
}

/// What `bit-cli webseed probe` reports for one source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceProbe {
    pub index: usize,
    pub url: String,
    pub scope: String,
    pub chunk_size: Size,
    pub steps: Vec<ConcurrencyStep>,
    /// The concurrency that reached the highest sustained throughput.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_concurrency: Option<usize>,
    /// The highest sustained throughput seen, in bytes per second.
    pub best_throughput: u64,
    pub best_throughput_human: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Measure ranged-GET latency and throughput as concurrency rises.
///
/// Every request reads real payload from inside the source's scope and drops
/// it, so this costs the mirror bandwidth. Nothing is written to disk and
/// nothing is hash-checked: this measures the transport, not the data.
pub async fn probe_source(
    binding: &Binding,
    layout: &Layout,
    info_hash: &str,
    concurrency_steps: &[usize],
    duration: Duration,
) -> SourceProbe {
    let chunk = binding.spec.limits.chunk_size.max(1);
    let mut report = SourceProbe {
        index: binding.index,
        url: binding.spec.url.clone(),
        scope: binding.scope.selector.clone(),
        chunk_size: Size(chunk),
        steps: Vec::new(),
        best_concurrency: None,
        best_throughput: 0,
        best_throughput_human: format_rate(0),
        error: None,
    };

    let client = match probe_client(binding) {
        Ok(client) => client,
        Err(e) => {
            report.error = Some(e.to_string());
            return report;
        }
    };
    // Every offset the probe reads from is inside the scope, so a partial
    // mirror is measured on the part it actually holds.
    let offsets = probe_offsets(binding, layout, info_hash, chunk);
    if offsets.is_empty() {
        report.error = Some("the scope holds no readable range to probe".to_string());
        return report;
    }

    let per_step = duration / (concurrency_steps.len().max(1) as u32);
    for &concurrency in concurrency_steps {
        let step = run_step(
            &client,
            binding,
            &offsets,
            chunk,
            concurrency.max(1),
            per_step,
        )
        .await;
        if step.throughput > report.best_throughput {
            report.best_throughput = step.throughput;
            report.best_throughput_human = format_rate(step.throughput);
            report.best_concurrency = Some(step.concurrency);
        }
        report.steps.push(step);
    }
    report
}

/// Stat a `file:` source, as the HTTP path probes a mirror.
///
/// Range support is `yes` without asking: a positioned read on a local file
/// always works, and reporting `unknown` would make a working source look
/// doubtful. The length comes from the filesystem, so the check that catches
/// the wrong file is the same one that catches the wrong mirror.
fn test_local(
    binding: &Binding,
    request_url: &str,
    expected_length: u64,
    started: Instant,
) -> SourceTest {
    let path = match crate::webseed::local::path_of(request_url) {
        Ok(path) => path,
        Err(e) => {
            return SourceTest::failed(
                binding,
                request_url.to_string(),
                "READ",
                started.elapsed(),
                e.to_string(),
            );
        }
    };
    let length = match std::fs::metadata(&path) {
        Ok(meta) if meta.is_dir() => {
            return SourceTest::failed(
                binding,
                request_url.to_string(),
                "READ",
                started.elapsed(),
                format!("{} is a directory, not a file", path.display()),
            );
        }
        Ok(meta) => meta.len(),
        Err(e) => {
            return SourceTest::failed(
                binding,
                request_url.to_string(),
                "READ",
                started.elapsed(),
                format!("{}: {e}", path.display()),
            );
        }
    };
    let elapsed = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    // A file longer than the torrent says is fine: a composition can point at
    // a container the torrent's file is a prefix of. Shorter is not.
    let long_enough = length >= expected_length;
    SourceTest {
        index: binding.index,
        url: binding.spec.url.clone(),
        request_url: request_url.to_string(),
        origin: binding.spec.origin.as_str().to_string(),
        scope: binding.scope.selector.clone(),
        mode: binding.spec.mode.as_str().to_string(),
        ok: long_enough,
        method: "READ",
        status: None,
        resolved_url: None,
        redirects: Vec::new(),
        range_support: RangeSupport::Yes,
        content_length: Some(length),
        expected_length,
        length_matches: Some(length == expected_length),
        server: Some("local file".to_string()),
        http_version: "file".to_string(),
        tls: None,
        ttfb_ms: elapsed,
        total_ms: elapsed,
        at: Timestamp::now().iso(),
        error: match long_enough {
            true => None,
            false => Some(format!(
                "{} is {length} bytes and the torrent says the file is {expected_length}",
                path.display()
            )),
        },
    }
}

/// The `(url, offset, length)` triples a probe reads from.
///
/// Reading the same offset repeatedly would measure the mirror's cache rather
/// than the mirror, so the probe walks distinct in-scope windows and wraps
/// around when it runs out.
fn probe_offsets(
    binding: &Binding,
    layout: &Layout,
    info_hash: &str,
    chunk: u64,
) -> Vec<(String, u64, u64)> {
    let mut out = Vec::new();
    for span in binding.scope.spans.spans() {
        let mut pos = span.start;
        while pos < span.end && out.len() < 64 {
            let length = chunk.min(span.end - pos);
            if let Ok(requests) = binding.request_urls(layout, info_hash, pos..pos + length) {
                for request in requests {
                    out.push((request.url, request.file_offset, request.length));
                }
            }
            pos += length;
        }
    }
    out
}

/// One concurrency step of a sweep.
async fn run_step(
    client: &reqwest::Client,
    binding: &Binding,
    offsets: &[(String, u64, u64)],
    chunk: u64,
    concurrency: usize,
    duration: Duration,
) -> ConcurrencyStep {
    let deadline = Instant::now() + duration;
    let total = Arc::new(std::sync::Mutex::new(Samples::default()));
    let next = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let mut workers = tokio::task::JoinSet::new();
    for _ in 0..concurrency {
        let client = client.clone();
        let headers = headers(binding);
        let auth = binding.spec.auth.clone();
        let offsets = offsets.to_vec();
        let total = total.clone();
        let next = next.clone();
        workers.spawn(async move {
            while Instant::now() < deadline {
                let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % offsets.len();
                let (url, offset, length) = &offsets[index];
                let began = Instant::now();
                // A local source reads the same range off the disk. The curve
                // then says whether concurrency helps the filesystem, which is
                // the same question and the same shape of answer.
                if crate::webseed::local::is_file_url(url) {
                    let (_, read) = crate::webseed::local::read_range(url, *offset, *length).await;
                    let ttfb = began.elapsed();
                    let (bytes, failed) = match read {
                        Ok(data) => (data.len() as u64, false),
                        Err(_) => (0, true),
                    };
                    let mut samples = total.lock().unwrap_or_else(|e| e.into_inner());
                    samples.record(began.elapsed(), ttfb, bytes, failed);
                    continue;
                }
                let mut request = client
                    .get(url)
                    .headers(headers.clone())
                    .header(RANGE, format!("bytes={}-{}", offset, offset + length - 1));
                if let Auth::Basic { user, password } = &auth {
                    request = request.basic_auth(user, Some(password));
                }
                let outcome = request.send().await;
                let ttfb = began.elapsed();
                let (bytes, failed) = match outcome {
                    Ok(response) if response.status().is_success() => {
                        match response.bytes().await {
                            Ok(body) => (body.len() as u64, false),
                            Err(_) => (0, true),
                        }
                    }
                    _ => (0, true),
                };
                let elapsed = began.elapsed();
                let mut samples = total.lock().unwrap_or_else(|e| e.into_inner());
                samples.record(elapsed, ttfb, bytes, failed);
            }
        });
    }
    let started = Instant::now();
    while workers.join_next().await.is_some() {}
    let elapsed = started.elapsed();

    let samples = std::mem::take(&mut *total.lock().unwrap_or_else(|e| e.into_inner()));
    samples.into_step(concurrency, elapsed, chunk)
}

/// Latency samples for one step.
struct Samples {
    total: Histogram<u64>,
    ttfb: Histogram<u64>,
    bytes: u64,
    requests: u64,
    errors: u64,
}

impl Default for Samples {
    fn default() -> Self {
        Self {
            // Three significant figures over a range up to an hour, which is
            // far past any request worth waiting for.
            total: Histogram::new_with_bounds(1, 3_600_000, 3).unwrap_or_else(|_| {
                Histogram::new(3).expect("a histogram with default bounds always builds")
            }),
            ttfb: Histogram::new_with_bounds(1, 3_600_000, 3).unwrap_or_else(|_| {
                Histogram::new(3).expect("a histogram with default bounds always builds")
            }),
            bytes: 0,
            requests: 0,
            errors: 0,
        }
    }
}

impl Samples {
    fn record(&mut self, total: Duration, ttfb: Duration, bytes: u64, failed: bool) {
        let ms = |d: Duration| d.as_millis().clamp(1, 3_600_000) as u64;
        let _ = self.total.record(ms(total));
        let _ = self.ttfb.record(ms(ttfb));
        self.bytes += bytes;
        self.requests += 1;
        self.errors += u64::from(failed);
    }

    fn into_step(self, concurrency: usize, elapsed: Duration, _chunk: u64) -> ConcurrencyStep {
        let elapsed_ms = elapsed.as_millis().min(u128::from(u64::MAX)) as u64;
        let throughput = match elapsed_ms {
            0 => 0,
            ms => self.bytes.saturating_mul(1000) / ms,
        };
        ConcurrencyStep {
            concurrency,
            requests: self.requests,
            errors: self.errors,
            bytes: self.bytes,
            bytes_human: format_size(self.bytes),
            elapsed_ms,
            throughput,
            throughput_human: format_rate(throughput),
            p50_ms: self.total.value_at_quantile(0.50),
            p90_ms: self.total.value_at_quantile(0.90),
            p99_ms: self.total.value_at_quantile(0.99),
            p999_ms: self.total.value_at_quantile(0.999),
            max_ms: self.total.max(),
            ttfb_p50_ms: self.ttfb.value_at_quantile(0.50),
            ttfb_p99_ms: self.ttfb.value_at_quantile(0.99),
        }
    }
}

/// An HTTP client that does not follow redirects.
///
/// Following them in the client would hide the chain, and the chain is the
/// diagnostic: a mirror that redirects to a login page looks healthy until you
/// can see where it went.
fn probe_client(binding: &Binding) -> Result<reqwest::Client> {
    let limits = &binding.spec.limits;
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(limits.timeout())
        .connect_timeout(limits.connect_timeout())
        .user_agent(
            binding
                .spec
                .user_agent
                .clone()
                .unwrap_or_else(default_user_agent),
        )
        .build()
        .map_err(|e| Error::network(format!("cannot build an HTTP client: {e}")))
}

/// The source's own headers, as a probe should send them.
fn headers(binding: &Binding) -> reqwest::header::HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    for (name, value) in &binding.spec.headers {
        if let (Ok(name), Ok(value)) = (
            reqwest::header::HeaderName::try_from(name.as_str()),
            reqwest::header::HeaderValue::from_str(value),
        ) {
            headers.insert(name, value);
        }
    }
    if let Auth::Bearer { token } = &binding.spec.auth
        && let Ok(mut value) = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
    {
        value.set_sensitive(true);
        headers.insert(reqwest::header::AUTHORIZATION, value);
    }
    headers
}

/// Make sure `rustls` has a cryptography provider before a config is built.
///
/// `rustls` 0.23 refuses to pick one on its own and panics when a
/// `ClientConfig` is built without one. `reqwest` installs one for the
/// connections it makes, but the TLS probe opens its own connection through
/// `tokio-rustls` and gets nothing from `reqwest`, so every HTTPS source
/// panicked here rather than reporting its cipher suite. Installing it once
/// per process, from any entry point that needs it, is what fixes that.
///
/// A second call returns `Err` because something already installed one, which
/// is the outcome this wants either way.
pub fn install_crypto_provider() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Resolve a `Location` header against the URL it came from.
pub fn resolve_redirect(from: &str, location: &str) -> Result<String> {
    let base =
        url::Url::parse(from).map_err(|e| Error::network(format!("{from} is not a URL: {e}")))?;
    base.join(location).map(|url| url.to_string()).map_err(|e| {
        Error::network(format!(
            "{from} redirected to `{location}`, which is not a URL: {e}"
        ))
    })
}

/// Open a TLS connection and report what was negotiated.
///
/// `reqwest` does not expose the protocol version or the cipher suite, and
/// "which TLS did my CDN actually give me" is a real question when a mirror is
/// slow. So the probe opens one connection of its own to find out and closes
/// it immediately. It carries no request and no payload.
pub async fn tls_report(url: &str) -> Result<TlsReport> {
    tls_report_within(url, DEFAULT_TLS_PROBE_TIMEOUT).await
}

/// How long a TLS probe waits before giving up.
///
/// A probe is a diagnostic, not a transfer. Ten seconds is longer than any
/// reachable server takes to complete a handshake and short enough that a
/// mirror that accepts a connection and then says nothing does not hold the
/// command open forever.
pub const DEFAULT_TLS_PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// [`tls_report`] with an explicit deadline covering the whole exchange.
///
/// The deadline covers connect and handshake together rather than each
/// separately, because the failure being defended against is a server that
/// accepts the connection and then never finishes the handshake. Timing them
/// apart would let that case wait twice.
pub async fn tls_report_within(url: &str, timeout: Duration) -> Result<TlsReport> {
    tokio::time::timeout(timeout, tls_report_inner(url))
        .await
        .unwrap_or_else(|_| {
            Err(Error::timeout(format!(
                "{url}: no TLS handshake within {}ms",
                timeout.as_millis()
            )))
        })
}

async fn tls_report_inner(url: &str) -> Result<TlsReport> {
    use rustls_pki_types::ServerName;
    use tokio::net::TcpStream;
    use tokio_rustls::TlsConnector;

    install_crypto_provider();

    let parsed =
        url::Url::parse(url).map_err(|e| Error::network(format!("{url} is not a URL: {e}")))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| Error::network(format!("{url} has no host")))?
        .to_string();
    let port = parsed.port_or_known_default().unwrap_or(443);

    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let mut config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    let began = Instant::now();
    let stream = TcpStream::connect((host.as_str(), port))
        .await
        .map_err(|e| Error::network(format!("{host}:{port}: {e}")))?;
    let connect_ms = began.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

    let server_name = ServerName::try_from(host.clone())
        .map_err(|e| Error::network(format!("{host} is not a valid server name: {e}")))?;
    let handshake_began = Instant::now();
    let tls = TlsConnector::from(Arc::new(config))
        .connect(server_name, stream)
        .await
        .map_err(|e| Error::network(format!("{host}:{port}: TLS handshake failed: {e}")))?;
    let handshake_ms = handshake_began
        .elapsed()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;

    let (_, connection) = tls.get_ref();
    Ok(TlsReport {
        version: connection
            .protocol_version()
            .map(|v| format!("{v:?}"))
            .unwrap_or_else(|| "unknown".to_string()),
        cipher_suite: connection
            .negotiated_cipher_suite()
            .map(|suite| format!("{:?}", suite.suite()))
            .unwrap_or_else(|| "unknown".to_string()),
        server_name: host,
        alpn: connection
            .alpn_protocol()
            .map(|p| String::from_utf8_lossy(p).into_owned()),
        connect_ms,
        handshake_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webseed::binding::{BindingSet, Origin, SourceSpec};
    use crate::webseed::scope::Scope;

    fn layout() -> Layout {
        Layout::from_lengths(
            "movie.bin",
            false,
            32 * 1024,
            [("movie.bin".to_string(), 320 * 1024u64)],
        )
    }

    fn binding(scope: &str, chunk: u64) -> Binding {
        let layout = layout();
        let mut spec = SourceSpec::new("https://mirror.example.com/movie.bin", Origin::CommandLine)
            .with_scope(Scope::parse(scope).unwrap());
        spec.limits.chunk_size = chunk;
        BindingSet::resolve(&layout, &"0".repeat(40), &[spec])
            .unwrap()
            .bindings
            .remove(0)
    }

    #[test]
    fn a_relative_redirect_resolves_against_the_url_it_came_from() {
        assert_eq!(
            resolve_redirect("https://a.example.com/pub/x.iso", "/mirror/x.iso").unwrap(),
            "https://a.example.com/mirror/x.iso"
        );
        assert_eq!(
            resolve_redirect("https://a.example.com/pub/x.iso", "y.iso").unwrap(),
            "https://a.example.com/pub/y.iso"
        );
    }

    #[test]
    fn an_absolute_redirect_replaces_the_whole_url() {
        assert_eq!(
            resolve_redirect("https://a.example.com/x", "https://b.example.com/y").unwrap(),
            "https://b.example.com/y"
        );
    }

    #[test]
    fn a_redirect_to_nonsense_is_an_error_rather_than_a_bad_url() {
        assert!(resolve_redirect("https://a.example.com/x", "http://[bad").is_err());
    }

    #[test]
    fn probe_offsets_stay_inside_the_scope() {
        let layout = layout();
        let binding = binding("piece:5-", 64 * 1024);
        let offsets = probe_offsets(&binding, &layout, &"0".repeat(40), 64 * 1024);
        assert!(!offsets.is_empty());
        for (_, offset, length) in &offsets {
            assert!(
                *offset >= 5 * 32 * 1024,
                "offset {offset} is before the scope"
            );
            assert!(
                offset + length <= 320 * 1024,
                "offset {offset} runs past the payload"
            );
        }
    }

    #[test]
    fn probe_offsets_walk_distinct_windows_rather_than_hammering_one() {
        let layout = layout();
        let binding = binding("*", 64 * 1024);
        let offsets = probe_offsets(&binding, &layout, &"0".repeat(40), 64 * 1024);
        let distinct: std::collections::HashSet<u64> = offsets.iter().map(|(_, o, _)| *o).collect();
        assert!(
            distinct.len() > 1,
            "measuring one offset measures the mirror's cache"
        );
    }

    #[test]
    fn a_chunk_larger_than_the_payload_still_yields_one_offset() {
        let layout = layout();
        let binding = binding("*", 64 * crate::units::MIB);
        let offsets = probe_offsets(&binding, &layout, &"0".repeat(40), 64 * crate::units::MIB);
        assert_eq!(offsets.len(), 1);
        assert_eq!(offsets[0].1, 0);
        assert_eq!(
            offsets[0].2,
            320 * 1024,
            "clamped to the payload, not the chunk"
        );
    }

    #[test]
    fn probe_offsets_stop_before_they_become_a_load_test() {
        // A large payload with a small chunk would otherwise enumerate every
        // window in the torrent, which is a download rather than a probe.
        let layout = Layout::from_lengths(
            "big.bin",
            false,
            32 * 1024,
            [("big.bin".to_string(), 64 * crate::units::MIB)],
        );
        let mut spec = SourceSpec::new("https://mirror.example.com/big.bin", Origin::CommandLine);
        spec.limits.chunk_size = 64 * 1024;
        let set = BindingSet::resolve(&layout, &"0".repeat(40), &[spec]).unwrap();
        let offsets = probe_offsets(&set.bindings[0], &layout, &"0".repeat(40), 64 * 1024);
        assert!(
            offsets.len() <= 64,
            "a probe samples the payload, it does not walk it"
        );
    }

    #[test]
    fn range_support_has_stable_names() {
        assert_eq!(RangeSupport::Yes.as_str(), "yes");
        assert_eq!(RangeSupport::No.as_str(), "no");
        assert_eq!(RangeSupport::Unknown.as_str(), "unknown");
    }

    #[test]
    fn latency_samples_produce_percentiles_and_a_throughput() {
        let mut samples = Samples::default();
        for ms in 1..=100u64 {
            samples.record(
                Duration::from_millis(ms),
                Duration::from_millis(ms / 2),
                1000,
                false,
            );
        }
        let step = samples.into_step(4, Duration::from_secs(1), 1024);
        assert_eq!(step.requests, 100);
        assert_eq!(step.errors, 0);
        assert_eq!(step.bytes, 100_000);
        assert_eq!(step.throughput, 100_000, "100 KB in one second");
        assert!(
            step.p50_ms >= 49 && step.p50_ms <= 52,
            "p50 was {}",
            step.p50_ms
        );
        assert!(step.p99_ms >= 98, "p99 was {}", step.p99_ms);
        assert_eq!(step.max_ms, 100);
        assert!(
            step.ttfb_p50_ms < step.p50_ms,
            "time to first byte cannot exceed the total"
        );
    }

    #[test]
    fn a_failed_request_counts_as_an_error_and_contributes_no_bytes() {
        let mut samples = Samples::default();
        samples.record(Duration::from_millis(5), Duration::from_millis(5), 0, true);
        samples.record(
            Duration::from_millis(5),
            Duration::from_millis(5),
            500,
            false,
        );
        let step = samples.into_step(1, Duration::from_secs(1), 1024);
        assert_eq!(step.requests, 2);
        assert_eq!(step.errors, 1);
        assert_eq!(step.bytes, 500);
    }

    #[test]
    fn a_step_with_no_elapsed_time_reports_zero_rather_than_dividing_by_zero() {
        let step = Samples::default().into_step(1, Duration::ZERO, 1024);
        assert_eq!(step.throughput, 0);
        assert_eq!(step.requests, 0);
    }

    /// Building a `rustls` client config panics unless a cryptography provider
    /// is installed for the process, and `rustls` refuses to pick one on its
    /// own. Every HTTPS source went through this and panicked, and nothing
    /// caught it because every test until now used loopback HTTP.
    ///
    /// This asserts the config builds, which is the exact call that panicked.
    /// It needs no network.
    #[test]
    fn a_tls_config_builds_without_a_provider_being_installed_by_hand() {
        install_crypto_provider();
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        assert!(!config.alpn_protocols.iter().any(Vec::is_empty));
    }

    #[test]
    fn installing_the_provider_twice_is_not_an_error() {
        install_crypto_provider();
        install_crypto_provider();
        install_crypto_provider();
    }

    #[tokio::test]
    async fn a_tls_probe_of_a_closed_port_reports_an_error_rather_than_panicking() {
        // Port 1 on loopback has nothing listening, so this reaches the TLS
        // setup and fails at the connection, which is what proves the setup
        // itself did not panic.
        let error = tls_report("https://127.0.0.1:1/").await.unwrap_err();
        assert_eq!(error.code(), crate::exit::ExitCode::Network);
    }

    /// A server that accepts a connection and then says nothing must not hold
    /// the command open. `webseed test` against a real mirror hung here with
    /// no deadline at all before this.
    #[tokio::test]
    async fn a_tls_probe_gives_up_on_a_server_that_never_completes_a_handshake() {
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        // Accept and then hold the connection without speaking TLS.
        tokio::spawn(async move {
            let mut held = Vec::new();
            while let Ok((stream, _)) = listener.accept().await {
                held.push(stream);
            }
        });

        let began = Instant::now();
        let error = tls_report_within(
            &format!("https://127.0.0.1:{port}/"),
            Duration::from_millis(300),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code(), crate::exit::ExitCode::Timeout);
        assert!(
            began.elapsed() < Duration::from_secs(5),
            "the probe waited {:?}, which is not a deadline",
            began.elapsed()
        );
    }
}
