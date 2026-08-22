use arc_swap::ArcSwap;
use arc_swap::ArcSwapOption;
use governor::DefaultDirectRateLimiter as RateLimiter;
use governor::Quota;
use librqbit_core::hash_id::Id20;
use serde::Deserialize;
use serde::Serialize;
use std::num::NonZeroU32;
use std::sync::Arc;

/// The leading bytes of a peer id, which is how a BitTorrent client says who
/// it is. Azureus-style ids put the client in the first eight.
pub type PeerIdPrefix = [u8; 8];

#[derive(Default, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LimitsConfig {
    pub upload_bps: Option<NonZeroU32>,
    pub download_bps: Option<NonZeroU32>,
}

struct Limit {
    limiter: ArcSwapOption<RateLimiter>,
    current_bps: std::sync::atomic::AtomicU32,
}

impl Limit {
    fn new_inner(bps: Option<NonZeroU32>) -> Option<Arc<RateLimiter>> {
        let bps = bps?;
        Some(Arc::new(RateLimiter::direct(Quota::per_second(bps))))
    }

    fn new(bps: Option<NonZeroU32>) -> Self {
        use std::sync::atomic::AtomicU32;
        Self {
            limiter: ArcSwapOption::new(Self::new_inner(bps)),
            current_bps: AtomicU32::new(bps.map(|v| v.get()).unwrap_or(0)),
        }
    }

    async fn acquire(&self, size: NonZeroU32) -> crate::Result<()> {
        let lim = self.limiter.load().clone();
        if let Some(rl) = lim.as_ref() {
            rl.until_n_ready(size).await?;
        }
        Ok(())
    }

    fn set(&self, limit: Option<NonZeroU32>) {
        use std::sync::atomic::Ordering;
        let new = Self::new_inner(limit);
        self.limiter.swap(new);
        self.current_bps
            .store(limit.map(|v| v.get()).unwrap_or(0), Ordering::Relaxed);
    }

    fn get(&self) -> Option<NonZeroU32> {
        use std::sync::atomic::Ordering;
        NonZeroU32::new(self.current_bps.load(Ordering::Relaxed))
    }
}

pub struct Limits {
    down: Limit,
    up: Limit,
    /// A second download limit that some peers do not pass through.
    ///
    /// `down` bounds everything, which is what a total does. This bounds the
    /// swarm on its own, so a client that is also feeding the session from a
    /// source of its own can cap peers without capping that source. Off by
    /// default, so a session that never sets it behaves exactly as before.
    ///
    /// There is no upload counterpart and that is not an oversight: a source
    /// bridged into the session is a seed. It never sends `Interested` and
    /// never requests, so nothing is ever uploaded to it and the upload limit
    /// already applies to peers alone.
    peer_down: Limit,
    /// Peer id prefixes that `peer_down` does not apply to.
    ///
    /// A prefix rather than a whole id, because a client's own bridge picks a
    /// fresh id per connection and only the first eight bytes identify it. A
    /// prefix rather than an address, because a bridge dials in from an
    /// ephemeral port and reconnects on a new one.
    exempt: ArcSwap<Vec<PeerIdPrefix>>,
}

impl Limits {
    pub fn new(config: LimitsConfig) -> Self {
        Self {
            down: Limit::new(config.download_bps),
            up: Limit::new(config.upload_bps),
            peer_down: Limit::new(None),
            exempt: ArcSwap::from_pointee(Vec::new()),
        }
    }

    /// Whether `peer_down` applies to this peer.
    pub fn is_exempt_from_peer_limits(&self, peer_id: &Id20) -> bool {
        let exempt = self.exempt.load();
        if exempt.is_empty() {
            return false;
        }
        exempt.iter().any(|prefix| peer_id.0.starts_with(prefix))
    }

    /// Like `prepare_for_download`, and also charges the peer limit unless
    /// this peer is exempt from it.
    ///
    /// The total is charged either way. An exemption is from the peer limit
    /// and never from the session's.
    pub async fn prepare_for_download_from(
        &self,
        peer_id: Option<&Id20>,
        len: NonZeroU32,
    ) -> crate::Result<()> {
        self.down.acquire(len).await?;
        let exempt = peer_id.is_some_and(|id| self.is_exempt_from_peer_limits(id));
        if !exempt {
            self.peer_down.acquire(len).await?;
        }
        Ok(())
    }

    pub fn set_peer_download_bps(&self, bps: Option<NonZeroU32>) {
        self.peer_down.set(bps);
    }

    pub fn get_peer_download_bps(&self) -> Option<NonZeroU32> {
        self.peer_down.get()
    }

    pub fn set_exempt_peer_prefixes(&self, prefixes: Vec<PeerIdPrefix>) {
        self.exempt.store(Arc::new(prefixes));
    }

    pub fn get_exempt_peer_prefixes(&self) -> Arc<Vec<PeerIdPrefix>> {
        self.exempt.load_full()
    }

    pub async fn prepare_for_upload(&self, len: NonZeroU32) -> crate::Result<()> {
        self.up.acquire(len).await
    }

    pub async fn prepare_for_download(&self, len: NonZeroU32) -> crate::Result<()> {
        self.down.acquire(len).await
    }

    pub fn set_upload_bps(&self, bps: Option<NonZeroU32>) {
        self.up.set(bps);
    }

    pub fn set_download_bps(&self, bps: Option<NonZeroU32>) {
        self.down.set(bps);
    }

    pub fn get_upload_bps(&self) -> Option<NonZeroU32> {
        self.up.get()
    }

    pub fn get_download_bps(&self) -> Option<NonZeroU32> {
        self.down.get()
    }

    pub fn get_config(&self) -> LimitsConfig {
        LimitsConfig {
            upload_bps: self.get_upload_bps(),
            download_bps: self.get_download_bps(),
        }
    }
}
