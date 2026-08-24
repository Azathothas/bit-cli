use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::torrent::{TorrentMeta, MAX_PIECE_LENGTH};
use crate::{sha1, sha256};

pub const BLOCK_LEN: u32 = 16 * 1024;
pub const PRIORITY_SKIP: u8 = 0;
#[allow(dead_code)]
pub const PRIORITY_LOW: u8 = 1;
pub const PRIORITY_NORMAL: u8 = 2;
pub const PRIORITY_HIGH: u8 = 3;

#[derive(Debug, Clone)]
pub enum PieceHash {
    Sha1([u8; 20]),
    Sha256 {
        root: [u8; 32],
        merkle_length: u32,
        data_length: u32,
    },
    Hybrid {
        sha1: [u8; 20],
        sha256: [u8; 32],
        merkle_length: u32,
        v2_data_length: u32,
    },
}

impl PieceHash {
    pub fn verify(&self, data: &[u8]) -> bool {
        match self {
            PieceHash::Sha1(expected) => sha1::sha1(data) == *expected,
            PieceHash::Sha256 {
                root,
                merkle_length,
                data_length,
            } => {
                data.len() == *data_length as usize
                    && sha256::merkle_piece_root(data, *merkle_length) == Some(*root)
            }
            PieceHash::Hybrid {
                sha1: expected_sha1,
                sha256: expected_sha256,
                merkle_length,
                v2_data_length,
            } => {
                sha1::sha1(data) == *expected_sha1
                    && data.get(..*v2_data_length as usize).is_some_and(|v2_data| {
                        sha256::merkle_piece_root(v2_data, *merkle_length) == Some(*expected_sha256)
                    })
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockState {
    Missing,
    Requested,
    Complete,
}

#[derive(Debug, Clone)]
pub struct Piece {
    pub index: u32,
    pub hash: PieceHash,
    pub offset: u64,
    pub length: u32,
    blocks: Vec<BlockState>,
    priority: u8,
    wanted: bool,
    verified: bool,
}

#[derive(Debug)]
pub struct PieceManager {
    pieces: Vec<Piece>,
    availability: Vec<u32>,
    reserved_by: Vec<Option<u64>>,
    reservation_time: Vec<Option<Instant>>,
    sequential: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct BlockRequest {
    pub index: u32,
    pub begin: u32,
    pub length: u32,
}

#[derive(Debug)]
pub struct PieceBuffer {
    index: u32,
    length: u32,
    data: Vec<u8>,
    blocks: Vec<u8>,
    complete: usize,
    _budget_reservation: Option<PieceBufferReservation>,
}

#[derive(Debug)]
pub struct PieceBufferBudget {
    limit: usize,
    used: AtomicUsize,
}

#[derive(Clone, Debug)]
pub struct PieceBufferBudgets {
    global: Arc<PieceBufferBudget>,
    torrent: Arc<PieceBufferBudget>,
}

#[derive(Debug)]
struct BudgetCounterPermit {
    budget: Arc<PieceBufferBudget>,
    bytes: usize,
}

#[derive(Debug)]
pub struct PieceBufferReservation {
    _torrent: BudgetCounterPermit,
    _global: BudgetCounterPermit,
}

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    InvalidPieceLength,
    InvalidPieces,
    InvalidBitfield,
    InvalidPiece,
    InvalidBlock,
    InvalidPriority,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidPieceLength => write!(f, "invalid piece length"),
            Error::InvalidPieces => write!(f, "invalid pieces"),
            Error::InvalidBitfield => write!(f, "invalid bitfield"),
            Error::InvalidPiece => write!(f, "invalid piece index"),
            Error::InvalidBlock => write!(f, "invalid block"),
            Error::InvalidPriority => write!(f, "invalid priority"),
        }
    }
}

impl std::error::Error for Error {}

impl PieceBufferBudget {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            used: AtomicUsize::new(0),
        }
    }

    fn try_acquire(self: &Arc<Self>, bytes: usize) -> Option<BudgetCounterPermit> {
        self.used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(bytes).filter(|next| *next <= self.limit)
            })
            .ok()?;
        Some(BudgetCounterPermit {
            budget: Arc::clone(self),
            bytes,
        })
    }

    #[cfg(test)]
    pub fn used(&self) -> usize {
        self.used.load(Ordering::Acquire)
    }
}

impl PieceBufferBudgets {
    pub fn new(global: Arc<PieceBufferBudget>, torrent: Arc<PieceBufferBudget>) -> Self {
        Self { global, torrent }
    }

    /// Reserves logical allocation bytes against both the per-torrent and
    /// process-wide limits. The returned non-cloneable guard releases both
    /// reservations when it is dropped.
    pub fn try_reserve(&self, bytes: usize) -> Option<PieceBufferReservation> {
        if bytes == 0 {
            return None;
        }
        let torrent = self.torrent.try_acquire(bytes)?;
        let global = self.global.try_acquire(bytes)?;
        Some(PieceBufferReservation {
            _torrent: torrent,
            _global: global,
        })
    }
}

impl Drop for BudgetCounterPermit {
    fn drop(&mut self) {
        let previous = self.budget.used.fetch_sub(self.bytes, Ordering::AcqRel);
        debug_assert!(previous >= self.bytes);
    }
}

impl PieceManager {
    pub fn new(meta: &TorrentMeta) -> Result<Self, Error> {
        let piece_length = meta.info.piece_length;
        if piece_length == 0 || piece_length > MAX_PIECE_LENGTH {
            return Err(Error::InvalidPieceLength);
        }
        let piece_length = piece_length as u32;

        let pieces = match meta.meta_version {
            1 => Self::build_v1_pieces(meta, piece_length, None)?,
            2 => Self::build_v2_pieces(meta, piece_length)?,
            3 => {
                let v2_pieces = Self::build_v2_pieces(meta, piece_length)?;
                Self::build_v1_pieces(meta, piece_length, Some(&v2_pieces))?
            }
            _ => return Err(Error::InvalidPieces),
        };
        let piece_count = pieces.len();
        if piece_count == 0 || piece_count > u32::MAX as usize {
            return Err(Error::InvalidPieces);
        }

        Ok(Self {
            pieces,
            availability: vec![0; piece_count],
            reserved_by: vec![None; piece_count],
            reservation_time: vec![None; piece_count],
            sequential: false,
        })
    }

    fn build_v1_pieces(
        meta: &TorrentMeta,
        piece_length: u32,
        v2_pieces: Option<&[Piece]>,
    ) -> Result<Vec<Piece>, Error> {
        let piece_count = meta.info.pieces.len();
        if piece_count == 0 {
            return Err(Error::InvalidPieces);
        }

        let total_length = meta
            .info
            .checked_total_length()
            .ok_or(Error::InvalidPieces)?;
        if total_length == 0 {
            return Err(Error::InvalidPieces);
        }

        let min_total = (piece_count as u64 - 1)
            .checked_mul(piece_length as u64)
            .ok_or(Error::InvalidPieces)?;
        if total_length < min_total {
            return Err(Error::InvalidPieces);
        }
        let last_len = total_length - min_total;
        if last_len == 0 || last_len > piece_length as u64 {
            return Err(Error::InvalidPieces);
        }

        if let Some(v2_pieces) = v2_pieces {
            if v2_pieces.len() != piece_count {
                return Err(Error::InvalidPieces);
            }
        }

        let mut pieces = Vec::with_capacity(piece_count);
        for (index, sha1_hash) in meta.info.pieces.iter().copied().enumerate() {
            let length = if index + 1 == piece_count {
                last_len as u32
            } else {
                piece_length
            };
            let offset = (index as u64)
                .checked_mul(piece_length as u64)
                .ok_or(Error::InvalidPieces)?;
            let hash = if let Some(v2_pieces) = v2_pieces {
                if v2_pieces[index].offset != offset {
                    return Err(Error::InvalidPieces);
                }
                let (root, merkle_length, v2_data_length) = match &v2_pieces[index].hash {
                    PieceHash::Sha256 {
                        root,
                        merkle_length,
                        data_length,
                    } => (*root, *merkle_length, *data_length),
                    _ => return Err(Error::InvalidPieces),
                };
                PieceHash::Hybrid {
                    sha1: sha1_hash,
                    sha256: root,
                    merkle_length,
                    v2_data_length,
                }
            } else {
                PieceHash::Sha1(sha1_hash)
            };
            let blocks = block_count(length);
            pieces.push(Piece {
                index: index as u32,
                hash,
                offset,
                length,
                blocks: vec![BlockState::Missing; blocks],
                priority: PRIORITY_NORMAL,
                wanted: true,
                verified: false,
            });
        }

        Ok(pieces)
    }

    fn build_v2_pieces(meta: &TorrentMeta, piece_length: u32) -> Result<Vec<Piece>, Error> {
        if piece_length < 16 * 1024 || !piece_length.is_power_of_two() {
            return Err(Error::InvalidPieceLength);
        }
        let piece_length_u64 = piece_length as u64;
        let mut pieces = Vec::new();
        let mut file_offset = 0u64;
        for entry in &meta.info.file_tree {
            if entry.length == 0 {
                continue;
            }
            let root = entry.pieces_root.ok_or(Error::InvalidPieces)?;
            let roots: Vec<[u8; 32]> = if entry.length <= piece_length_u64 {
                vec![root]
            } else {
                let (_, hashes) = meta
                    .piece_layers
                    .iter()
                    .find(|(key, _)| key.as_slice() == root.as_slice())
                    .ok_or(Error::InvalidPieces)?;
                if u64::try_from(hashes.len()).ok() != Some(entry.length.div_ceil(piece_length_u64))
                {
                    return Err(Error::InvalidPieces);
                }
                hashes.clone()
            };

            for (file_piece_index, root) in roots.into_iter().enumerate() {
                if pieces.len() > u32::MAX as usize {
                    return Err(Error::InvalidPieces);
                }
                let within_file = (file_piece_index as u64)
                    .checked_mul(piece_length_u64)
                    .ok_or(Error::InvalidPieces)?;
                let remaining = entry
                    .length
                    .checked_sub(within_file)
                    .ok_or(Error::InvalidPieces)?;
                let length = remaining.min(piece_length_u64) as u32;
                let merkle_length = if entry.length <= piece_length_u64 {
                    v2_tree_length(length).ok_or(Error::InvalidPieces)?
                } else {
                    piece_length
                };
                let offset = file_offset
                    .checked_add(within_file)
                    .ok_or(Error::InvalidPieces)?;
                pieces.push(Piece {
                    index: pieces.len() as u32,
                    hash: PieceHash::Sha256 {
                        root,
                        merkle_length,
                        data_length: length,
                    },
                    offset,
                    length,
                    blocks: vec![BlockState::Missing; block_count(length)],
                    priority: PRIORITY_NORMAL,
                    wanted: true,
                    verified: false,
                });
            }
            let file_piece_count = entry.length.div_ceil(piece_length_u64);
            file_offset = file_offset
                .checked_add(
                    file_piece_count
                        .checked_mul(piece_length_u64)
                        .ok_or(Error::InvalidPieces)?,
                )
                .ok_or(Error::InvalidPieces)?;
        }
        Ok(pieces)
    }

    pub fn piece_count(&self) -> usize {
        self.pieces.len()
    }

    pub fn completed_pieces(&self) -> usize {
        self.pieces
            .iter()
            .filter(|piece| piece.wanted && piece.verified)
            .count()
    }

    pub fn completed_bytes(&self) -> u64 {
        self.pieces
            .iter()
            .filter(|piece| piece.wanted && piece.verified)
            .map(|piece| piece.length as u64)
            .sum()
    }

    pub fn remaining_blocks(&self) -> usize {
        self.pieces
            .iter()
            .filter(|piece| piece.wanted)
            .map(|piece| piece.remaining_blocks())
            .sum()
    }

    pub fn is_complete(&self) -> bool {
        self.pieces
            .iter()
            .all(|piece| !piece.wanted || piece.verified)
    }

    pub fn reset_verified(&mut self) {
        for piece in &mut self.pieces {
            piece.verified = false;
            for block in &mut piece.blocks {
                *block = BlockState::Missing;
            }
        }
        for slot in &mut self.reserved_by {
            *slot = None;
        }
        for slot in &mut self.reservation_time {
            *slot = None;
        }
    }

    #[allow(dead_code)]
    pub fn next_missing_piece(&self) -> Option<u32> {
        let mut best = None;
        let mut best_priority = 0u8;
        for (idx, piece) in self.pieces.iter().enumerate() {
            if !piece.wanted || piece.verified || !piece.has_missing() {
                continue;
            }
            if piece.priority > best_priority {
                best_priority = piece.priority;
                best = Some(idx as u32);
            }
        }
        best
    }

    pub fn piece_length(&self, index: u32) -> Option<u32> {
        self.pieces.get(index as usize).map(|piece| piece.length)
    }

    pub fn piece_offset(&self, index: u32) -> Option<u64> {
        self.pieces.get(index as usize).map(|piece| piece.offset)
    }

    pub fn piece_hash(&self, index: u32) -> Option<&PieceHash> {
        self.pieces.get(index as usize).map(|piece| &piece.hash)
    }

    pub fn is_piece_complete(&self, index: u32) -> bool {
        self.pieces
            .get(index as usize)
            .map(|piece| piece.verified)
            .unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn is_piece_wanted(&self, index: u32) -> bool {
        self.pieces
            .get(index as usize)
            .map(|piece| piece.wanted)
            .unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn piece_priority(&self, index: u32) -> Option<u8> {
        self.pieces.get(index as usize).map(|piece| piece.priority)
    }

    pub fn wanted_bytes(&self) -> u64 {
        self.pieces
            .iter()
            .filter(|piece| piece.wanted)
            .map(|piece| piece.length as u64)
            .sum()
    }

    pub fn wanted_pieces(&self) -> usize {
        self.pieces.iter().filter(|piece| piece.wanted).count()
    }

    pub fn set_sequential(&mut self, sequential: bool) {
        self.sequential = sequential;
    }

    pub fn set_piece_priorities(&mut self, priorities: &[u8]) -> Result<(), Error> {
        if priorities.len() != self.pieces.len() {
            return Err(Error::InvalidPieces);
        }
        if priorities.iter().any(|priority| *priority > PRIORITY_HIGH) {
            return Err(Error::InvalidPriority);
        }
        for (idx, (piece, priority)) in self.pieces.iter_mut().zip(priorities.iter()).enumerate() {
            piece.priority = *priority;
            piece.wanted = *priority != PRIORITY_SKIP;
            if !piece.wanted {
                if let Some(reserved) = self.reserved_by.get_mut(idx) {
                    *reserved = None;
                }
                if let Some(reserved_at) = self.reservation_time.get_mut(idx) {
                    *reserved_at = None;
                }
            }
        }
        Ok(())
    }

    pub fn bitfield_len(&self) -> usize {
        self.pieces.len().div_ceil(8)
    }

    pub fn apply_peer_bitfield(&mut self, bitfield: &[u8]) -> Result<(), Error> {
        if bitfield.len() != self.bitfield_len() {
            return Err(Error::InvalidBitfield);
        }
        let total_bits = bitfield.len() * 8;
        let extra_bits = total_bits - self.pieces.len();
        if extra_bits > 0 {
            let mask = (1u8 << extra_bits) - 1;
            if bitfield[bitfield.len() - 1] & mask != 0 {
                return Err(Error::InvalidBitfield);
            }
        }
        for idx in 0..self.pieces.len() {
            if bitfield_has(bitfield, idx) {
                self.availability[idx] = self.availability[idx].saturating_add(1);
            }
        }
        Ok(())
    }

    pub fn apply_have(&mut self, index: u32) -> Result<(), Error> {
        let idx = index as usize;
        if idx >= self.pieces.len() {
            return Err(Error::InvalidPiece);
        }
        self.availability[idx] = self.availability[idx].saturating_add(1);
        Ok(())
    }

    pub fn reserve_piece_for_peer(
        &mut self,
        peer_id: u64,
        bitfield: &[u8],
        allow_reserved: bool,
    ) -> Option<u32> {
        if bitfield.len() != self.bitfield_len() {
            return None;
        }

        if self.sequential {
            // Sequential: pick lowest-index incomplete piece the peer has
            for (idx, piece) in self.pieces.iter().enumerate() {
                if piece.verified || !piece.has_missing() {
                    continue;
                }
                if !piece.wanted {
                    continue;
                }
                if !bitfield_has(bitfield, idx) {
                    continue;
                }
                if !allow_reserved && self.reserved_by[idx].is_some() {
                    continue;
                }
                if !allow_reserved {
                    self.reserved_by[idx] = Some(peer_id);
                    self.reservation_time[idx] = Some(Instant::now());
                }
                return Some(idx as u32);
            }
            return None;
        }

        let mut best_piece = None;
        let mut best_priority = 0u8;
        let mut best_rarity = u32::MAX;
        for (idx, piece) in self.pieces.iter().enumerate() {
            if piece.verified || !piece.has_missing() {
                continue;
            }
            if !piece.wanted {
                continue;
            }
            if !bitfield_has(bitfield, idx) {
                continue;
            }
            if !allow_reserved && self.reserved_by[idx].is_some() {
                continue;
            }
            let rarity = self.availability[idx];
            let priority = piece.priority;
            if best_piece.is_none()
                || priority > best_priority
                || (priority == best_priority && rarity < best_rarity)
            {
                best_priority = priority;
                best_rarity = rarity;
                best_piece = Some(idx);
            }
        }

        let idx = best_piece?;
        if !allow_reserved {
            self.reserved_by[idx] = Some(peer_id);
            self.reservation_time[idx] = Some(Instant::now());
        }
        Some(idx as u32)
    }

    pub fn has_needed_piece(&self, bitfield: &[u8]) -> bool {
        if bitfield.len() != self.bitfield_len() {
            return false;
        }

        self.pieces.iter().enumerate().any(|(idx, piece)| {
            piece.wanted && !piece.verified && piece.has_missing() && bitfield_has(bitfield, idx)
        })
    }

    pub fn release_piece(&mut self, peer_id: u64, index: u32) {
        let idx = index as usize;
        if idx >= self.reserved_by.len() {
            return;
        }
        if self.reserved_by[idx] == Some(peer_id) {
            self.reserved_by[idx] = None;
            self.reservation_time[idx] = None;
        }
    }

    pub fn clear_reservation(&mut self, index: u32) {
        let idx = index as usize;
        if idx < self.reserved_by.len() {
            self.reserved_by[idx] = None;
            self.reservation_time[idx] = None;
        }
    }

    /// Steal a piece that has been reserved by another peer for longer than
    /// `stale_threshold`. Returns the piece index if a stale reservation was
    /// found and reassigned to `peer_id`.
    pub fn steal_stale_piece(
        &mut self,
        peer_id: u64,
        bitfield: &[u8],
        stale_threshold: Duration,
    ) -> Option<u32> {
        if bitfield.len() != self.bitfield_len() {
            return None;
        }

        let now = Instant::now();
        let mut best_piece = None;
        let mut best_priority = 0u8;
        let mut best_rarity = u32::MAX;

        for (idx, piece) in self.pieces.iter().enumerate() {
            if piece.verified || !piece.has_missing() || !piece.wanted {
                continue;
            }
            if !bitfield_has(bitfield, idx) {
                continue;
            }
            // Only consider pieces reserved by a *different* peer
            match self.reserved_by[idx] {
                Some(owner) if owner != peer_id => {}
                _ => continue,
            }
            // Check if the reservation is stale
            let reserved_at = match self.reservation_time[idx] {
                Some(t) => t,
                None => continue,
            };
            if now.duration_since(reserved_at) < stale_threshold {
                continue;
            }

            let rarity = self.availability[idx];
            let priority = piece.priority;
            if best_piece.is_none()
                || priority > best_priority
                || (priority == best_priority && rarity < best_rarity)
            {
                best_priority = priority;
                best_rarity = rarity;
                best_piece = Some(idx);
            }
        }

        let idx = best_piece?;
        self.reserved_by[idx] = Some(peer_id);
        self.reservation_time[idx] = Some(now);
        Some(idx as u32)
    }

    pub fn next_request_for_piece(
        &mut self,
        index: u32,
        allow_duplicate: bool,
    ) -> Option<BlockRequest> {
        let piece = self.pieces.get_mut(index as usize)?;
        if !piece.wanted {
            return None;
        }
        let block_index = piece.next_requestable_block(allow_duplicate)?;
        let begin = block_index as u32 * BLOCK_LEN;
        let length = piece.block_length(block_index);
        if piece.blocks[block_index] == BlockState::Missing {
            piece.blocks[block_index] = BlockState::Requested;
        }
        Some(BlockRequest {
            index: piece.index,
            begin,
            length,
        })
    }

    pub fn remove_peer_bitfield(&mut self, bitfield: &[u8]) -> Result<(), Error> {
        if bitfield.len() != self.bitfield_len() {
            return Err(Error::InvalidBitfield);
        }
        for idx in 0..self.pieces.len() {
            if bitfield_has(bitfield, idx) {
                self.availability[idx] = self.availability[idx].saturating_sub(1);
            }
        }
        Ok(())
    }

    #[cfg(test)]
    pub fn select_next_request(&mut self, bitfield: &[u8]) -> Option<BlockRequest> {
        if bitfield.len() != self.bitfield_len() {
            return None;
        }

        let mut best_piece = None;
        let mut best_rarity = u32::MAX;
        for (idx, piece) in self.pieces.iter().enumerate() {
            if piece.verified || !piece.has_missing() {
                continue;
            }
            if !bitfield_has(bitfield, idx) {
                continue;
            }
            let rarity = self.availability[idx];
            if rarity < best_rarity {
                best_rarity = rarity;
                best_piece = Some(idx);
            }
        }

        let idx = best_piece?;
        let piece = &mut self.pieces[idx];
        let block_index = piece.next_missing_block()?;
        let begin = block_index as u32 * BLOCK_LEN;
        let length = piece.block_length(block_index);
        piece.blocks[block_index] = BlockState::Requested;
        Some(BlockRequest {
            index: piece.index,
            begin,
            length,
        })
    }

    pub fn mark_block_complete(
        &mut self,
        index: u32,
        begin: u32,
        length: u32,
    ) -> Result<bool, Error> {
        let piece = self
            .pieces
            .get_mut(index as usize)
            .ok_or(Error::InvalidPiece)?;
        if !begin.is_multiple_of(BLOCK_LEN) {
            return Err(Error::InvalidBlock);
        }
        let block_index = (begin / BLOCK_LEN) as usize;
        if block_index >= piece.blocks.len() {
            return Err(Error::InvalidBlock);
        }
        if piece.block_length(block_index) != length {
            return Err(Error::InvalidBlock);
        }
        if piece.blocks[block_index] == BlockState::Complete {
            return Ok(false);
        }
        piece.blocks[block_index] = BlockState::Complete;
        Ok(true)
    }

    pub fn mark_piece_complete(&mut self, index: u32) -> Result<bool, Error> {
        let idx = index as usize;
        let piece = self.pieces.get_mut(idx).ok_or(Error::InvalidPiece)?;
        let was_new = !piece.verified;
        piece.verified = true;
        for state in &mut piece.blocks {
            *state = BlockState::Complete;
        }
        self.reserved_by[idx] = None;
        self.reservation_time[idx] = None;
        Ok(was_new)
    }

    pub fn mark_block_missing(&mut self, index: u32, begin: u32) -> Result<(), Error> {
        let piece = self
            .pieces
            .get_mut(index as usize)
            .ok_or(Error::InvalidPiece)?;
        if !begin.is_multiple_of(BLOCK_LEN) {
            return Err(Error::InvalidBlock);
        }
        let block_index = (begin / BLOCK_LEN) as usize;
        if block_index >= piece.blocks.len() {
            return Err(Error::InvalidBlock);
        }
        if piece.blocks[block_index] != BlockState::Complete {
            piece.blocks[block_index] = BlockState::Missing;
        }
        Ok(())
    }

    pub fn reset_piece(&mut self, index: u32) -> Result<(), Error> {
        let idx = index as usize;
        let piece = self.pieces.get_mut(idx).ok_or(Error::InvalidPiece)?;
        piece.verified = false;
        for state in &mut piece.blocks {
            *state = BlockState::Missing;
        }
        self.reserved_by[idx] = None;
        self.reservation_time[idx] = None;
        Ok(())
    }
}

impl PieceBuffer {
    pub fn try_new(
        index: u32,
        length: u32,
        budgets: &PieceBufferBudgets,
    ) -> Result<Option<Self>, Error> {
        if length == 0 || length as u64 > MAX_PIECE_LENGTH {
            return Err(Error::InvalidPieceLength);
        }
        let blocks = block_count(length);
        let allocation_bytes = (length as usize)
            .checked_add(blocks)
            .ok_or(Error::InvalidPieceLength)?;
        let Some(reservation) = budgets.try_reserve(allocation_bytes) else {
            return Ok(None);
        };
        Self::allocate(index, length, Some(reservation)).map(Some)
    }

    #[cfg(test)]
    pub fn new(index: u32, length: u32) -> Result<Self, Error> {
        Self::allocate(index, length, None)
    }

    fn allocate(
        index: u32,
        length: u32,
        budget_reservation: Option<PieceBufferReservation>,
    ) -> Result<Self, Error> {
        if length == 0 || length as u64 > MAX_PIECE_LENGTH {
            return Err(Error::InvalidPieceLength);
        }
        let blocks = block_count(length);
        let mut data = Vec::new();
        data.try_reserve_exact(length as usize)
            .map_err(|_| Error::InvalidPieceLength)?;
        data.resize(length as usize, 0);
        let mut block_map = Vec::new();
        block_map
            .try_reserve_exact(blocks)
            .map_err(|_| Error::InvalidPieceLength)?;
        block_map.resize(blocks, 0);
        Ok(Self {
            index,
            length,
            data,
            blocks: block_map,
            complete: 0,
            _budget_reservation: budget_reservation,
        })
    }

    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn length(&self) -> u32 {
        self.length
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn add_block(&mut self, begin: u32, block: &[u8]) -> Result<bool, Error> {
        if !begin.is_multiple_of(BLOCK_LEN) {
            return Err(Error::InvalidBlock);
        }
        let block_index = (begin / BLOCK_LEN) as usize;
        if block_index >= self.blocks.len() {
            return Err(Error::InvalidBlock);
        }
        let expected_len = self.block_length(block_index) as usize;
        if block.len() != expected_len {
            return Err(Error::InvalidBlock);
        }
        let start = begin as usize;
        let end = start + block.len();
        if end > self.data.len() {
            return Err(Error::InvalidBlock);
        }
        if self.blocks[block_index] == 0 {
            self.data[start..end].copy_from_slice(block);
            self.blocks[block_index] = 1;
            self.complete += 1;
        }
        Ok(self.is_complete())
    }

    pub fn is_complete(&self) -> bool {
        self.complete == self.blocks.len()
    }

    fn block_length(&self, block_index: usize) -> u32 {
        let begin = block_index as u32 * BLOCK_LEN;
        let remaining = self.length.saturating_sub(begin);
        remaining.min(BLOCK_LEN)
    }
}

fn block_count(length: u32) -> usize {
    (length as u64).div_ceil(BLOCK_LEN as u64) as usize
}

fn v2_tree_length(data_length: u32) -> Option<u32> {
    let blocks = data_length.div_ceil(BLOCK_LEN);
    blocks.checked_next_power_of_two()?.checked_mul(BLOCK_LEN)
}

fn bitfield_has(bitfield: &[u8], index: usize) -> bool {
    let byte = bitfield[index / 8];
    let offset = index % 8;
    let mask = 0x80 >> offset;
    (byte & mask) != 0
}

impl Piece {
    fn has_missing(&self) -> bool {
        self.blocks.contains(&BlockState::Missing)
    }

    fn remaining_blocks(&self) -> usize {
        self.blocks
            .iter()
            .filter(|state| **state != BlockState::Complete)
            .count()
    }

    #[allow(dead_code)]
    fn next_missing_block(&self) -> Option<usize> {
        self.blocks
            .iter()
            .position(|state| *state == BlockState::Missing)
    }

    fn next_requestable_block(&self, allow_duplicate: bool) -> Option<usize> {
        if let Some(idx) = self
            .blocks
            .iter()
            .position(|state| *state == BlockState::Missing)
        {
            return Some(idx);
        }
        if allow_duplicate {
            return self
                .blocks
                .iter()
                .position(|state| *state == BlockState::Requested);
        }
        None
    }

    fn block_length(&self, block_index: usize) -> u32 {
        let begin = block_index as u32 * BLOCK_LEN;
        let remaining = self.length.saturating_sub(begin);
        remaining.min(BLOCK_LEN)
    }
}

#[cfg(test)]
mod priority_tests {
    use super::*;
    use crate::torrent::{InfoDict, TorrentMeta};

    fn dummy_meta() -> TorrentMeta {
        TorrentMeta {
            announce: None,
            announce_list: Vec::new(),
            url_list: Vec::new(),
            httpseeds: Vec::new(),
            info_hash: [0u8; 20],
            info_hash_v2: None,
            piece_layers: Vec::new(),
            meta_version: 1,
            info: InfoDict {
                name: b"test".to_vec(),
                piece_length: 16,
                pieces: vec![[1u8; 20], [2u8; 20]],
                length: Some(32),
                files: Vec::new(),
                private: false,
                file_tree: Vec::new(),
            },
        }
    }

    #[test]
    fn prefers_high_priority_piece() {
        let meta = dummy_meta();
        let mut manager = PieceManager::new(&meta).unwrap();
        manager
            .set_piece_priorities(&[PRIORITY_LOW, PRIORITY_HIGH])
            .unwrap();
        let bitfield = vec![0b1100_0000];
        let selected = manager.reserve_piece_for_peer(1, &bitfield, false);
        assert_eq!(selected, Some(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::torrent::{FileInfo, FileTreeEntry, InfoDict, TorrentMeta};

    fn dummy_meta(pieces: usize, piece_length: u64, total_length: u64) -> TorrentMeta {
        let mut hashes = Vec::with_capacity(pieces);
        for i in 0..pieces {
            let mut hash = [0u8; 20];
            hash[0] = i as u8;
            hashes.push(hash);
        }
        TorrentMeta {
            announce: None,
            announce_list: Vec::new(),
            url_list: Vec::new(),
            httpseeds: Vec::new(),
            info_hash: [0u8; 20],
            info_hash_v2: None,
            piece_layers: Vec::new(),
            meta_version: 1,
            info: InfoDict {
                name: b"dummy".to_vec(),
                piece_length,
                pieces: hashes,
                length: Some(total_length),
                files: Vec::new(),
                private: false,
                file_tree: Vec::new(),
            },
        }
    }

    #[test]
    fn selects_rarest_piece() {
        let meta = dummy_meta(3, 16 * 1024, 48 * 1024);
        let mut manager = PieceManager::new(&meta).unwrap();
        let mut bitfield = vec![0b1010_0000];
        manager.apply_peer_bitfield(&bitfield).unwrap();

        let req = manager.select_next_request(&bitfield).unwrap();
        assert_eq!(req.index, 0);

        bitfield[0] = 0b1110_0000;
        manager.apply_peer_bitfield(&bitfield).unwrap();
        let req = manager.select_next_request(&bitfield).unwrap();
        assert_eq!(req.index, 1);
    }

    #[test]
    fn last_piece_shorter() {
        let meta = dummy_meta(2, 16 * 1024, 20 * 1024);
        let manager = PieceManager::new(&meta).unwrap();
        assert_eq!(manager.pieces[0].length, 16 * 1024);
        assert_eq!(manager.pieces[1].length, 4 * 1024);
    }

    #[test]
    fn rejects_an_extra_zero_length_piece() {
        let meta = dummy_meta(2, 16 * 1024, 16 * 1024);
        assert!(matches!(
            PieceManager::new(&meta),
            Err(Error::InvalidPieces)
        ));
    }

    #[test]
    fn apply_peer_bitfield_rejects_extra_bits() {
        let meta = dummy_meta(9, 16 * 1024, 9 * 16 * 1024);
        let mut manager = PieceManager::new(&meta).unwrap();
        let bitfield = [0xFF, 0x40];
        assert!(matches!(
            manager.apply_peer_bitfield(&bitfield),
            Err(Error::InvalidBitfield)
        ));
    }

    #[test]
    fn sequential_mode_prefers_lowest_index_piece() {
        let meta = dummy_meta(3, 16 * 1024, 48 * 1024);
        let mut manager = PieceManager::new(&meta).unwrap();
        manager.set_sequential(true);
        manager.mark_piece_complete(0).unwrap();
        let bitfield = [0b1110_0000];
        assert_eq!(manager.reserve_piece_for_peer(7, &bitfield, false), Some(1));
    }

    #[test]
    fn skipping_piece_clears_existing_reservation() {
        let meta = dummy_meta(2, 16 * 1024, 32 * 1024);
        let mut manager = PieceManager::new(&meta).unwrap();
        let bitfield = [0b1100_0000];
        assert_eq!(manager.reserve_piece_for_peer(1, &bitfield, false), Some(0));
        manager
            .set_piece_priorities(&[PRIORITY_SKIP, PRIORITY_NORMAL])
            .unwrap();
        assert_eq!(manager.reserve_piece_for_peer(2, &bitfield, false), Some(1));
    }

    #[test]
    fn next_request_for_piece_allows_duplicate_when_enabled() {
        let meta = dummy_meta(1, 16 * 1024, 16 * 1024);
        let mut manager = PieceManager::new(&meta).unwrap();
        let first = manager.next_request_for_piece(0, false).unwrap();
        assert_eq!(first.begin, 0);
        assert!(manager.next_request_for_piece(0, false).is_none());
        let duplicate = manager.next_request_for_piece(0, true).unwrap();
        assert_eq!(duplicate.begin, 0);
        assert_eq!(duplicate.length, first.length);
    }

    #[test]
    fn mark_block_complete_validates_alignment_and_size() {
        let meta = dummy_meta(1, 16 * 1024, 16 * 1024);
        let mut manager = PieceManager::new(&meta).unwrap();
        assert!(matches!(
            manager.mark_block_complete(0, 1, 16 * 1024),
            Err(Error::InvalidBlock)
        ));
        assert!(matches!(
            manager.mark_block_complete(0, 0, 8),
            Err(Error::InvalidBlock)
        ));
        assert!(manager.mark_block_complete(0, 0, 16 * 1024).unwrap());
        assert!(!manager.mark_block_complete(0, 0, 16 * 1024).unwrap());
    }

    #[test]
    fn complete_requires_hash_verified_piece_not_only_received_blocks() {
        let meta = dummy_meta(1, 16 * 1024, 16 * 1024);
        let mut manager = PieceManager::new(&meta).unwrap();

        assert!(manager.mark_block_complete(0, 0, 16 * 1024).unwrap());

        assert!(!manager.is_piece_complete(0));
        assert!(!manager.is_complete());
        assert_eq!(manager.completed_pieces(), 0);

        manager.mark_piece_complete(0).unwrap();

        assert!(manager.is_piece_complete(0));
        assert!(manager.is_complete());
        assert_eq!(manager.completed_pieces(), 1);
    }

    #[test]
    fn piece_buffer_tracks_completion_across_blocks() {
        let mut buffer = PieceBuffer::new(2, BLOCK_LEN + 4).unwrap();
        let first = vec![1u8; BLOCK_LEN as usize];
        let second = vec![2u8; 4];
        assert!(!buffer.add_block(0, &first).unwrap());
        assert!(buffer.add_block(BLOCK_LEN, &second).unwrap());
        assert!(buffer.is_complete());
        assert_eq!(&buffer.data()[BLOCK_LEN as usize..], second.as_slice());
    }

    #[test]
    fn piece_buffer_budgets_backpressure_and_release_with_buffer_lifetime() {
        let allocation = BLOCK_LEN as usize + 1;
        let global = Arc::new(PieceBufferBudget::new(allocation));
        let torrent = Arc::new(PieceBufferBudget::new(allocation * 2));
        let budgets = PieceBufferBudgets::new(Arc::clone(&global), Arc::clone(&torrent));

        let first = PieceBuffer::try_new(0, BLOCK_LEN, &budgets)
            .unwrap()
            .unwrap();
        assert_eq!(global.used(), allocation);
        assert_eq!(torrent.used(), allocation);
        assert!(PieceBuffer::try_new(1, BLOCK_LEN, &budgets)
            .unwrap()
            .is_none());
        assert_eq!(global.used(), allocation);
        assert_eq!(torrent.used(), allocation);

        drop(first);
        assert_eq!(global.used(), 0);
        assert_eq!(torrent.used(), 0);
        let second = PieceBuffer::try_new(1, BLOCK_LEN, &budgets)
            .unwrap()
            .unwrap();
        drop(second);
        assert_eq!(global.used(), 0);
        assert_eq!(torrent.used(), 0);
    }

    #[test]
    fn generic_piece_buffer_reservation_releases_both_budgets() {
        let global = Arc::new(PieceBufferBudget::new(32));
        let torrent = Arc::new(PieceBufferBudget::new(16));
        let budgets = PieceBufferBudgets::new(Arc::clone(&global), Arc::clone(&torrent));

        let reservation = budgets.try_reserve(12).unwrap();
        assert_eq!(global.used(), 12);
        assert_eq!(torrent.used(), 12);
        assert!(budgets.try_reserve(5).is_none());
        assert_eq!(global.used(), 12);
        assert_eq!(torrent.used(), 12);

        drop(reservation);
        assert_eq!(global.used(), 0);
        assert_eq!(torrent.used(), 0);
        assert!(budgets.try_reserve(0).is_none());
    }

    #[test]
    fn priority_updates_are_atomic_on_validation_error() {
        let meta = dummy_meta(2, 16 * 1024, 32 * 1024);
        let mut manager = PieceManager::new(&meta).unwrap();
        assert!(matches!(
            manager.set_piece_priorities(&[PRIORITY_SKIP, PRIORITY_HIGH + 1]),
            Err(Error::InvalidPriority)
        ));
        assert_eq!(manager.piece_priority(0), Some(PRIORITY_NORMAL));
        assert!(manager.is_piece_wanted(0));
    }

    #[test]
    fn v2_files_have_aligned_offsets_and_merkle_verification() {
        let first_data = b"abc";
        let second_data = b"hello";
        let meta = TorrentMeta {
            announce: None,
            announce_list: Vec::new(),
            url_list: Vec::new(),
            httpseeds: Vec::new(),
            info_hash: [0; 20],
            info_hash_v2: Some([0; 32]),
            piece_layers: Vec::new(),
            meta_version: 2,
            info: InfoDict {
                name: b"v2".to_vec(),
                piece_length: 64 * 1024,
                pieces: Vec::new(),
                length: None,
                files: Vec::new(),
                private: false,
                file_tree: vec![
                    FileTreeEntry {
                        path: vec![b"a".to_vec()],
                        length: first_data.len() as u64,
                        pieces_root: Some(sha256::sha256(first_data)),
                    },
                    FileTreeEntry {
                        path: vec![b"b".to_vec()],
                        length: second_data.len() as u64,
                        pieces_root: Some(sha256::sha256(second_data)),
                    },
                ],
            },
        };

        let manager = PieceManager::new(&meta).unwrap();
        assert_eq!(manager.piece_count(), 2);
        assert_eq!(manager.piece_offset(0), Some(0));
        assert_eq!(manager.piece_offset(1), Some(64 * 1024));
        assert_eq!(manager.piece_length(0), Some(3));
        assert_eq!(manager.piece_length(1), Some(5));
        assert!(manager.piece_hash(0).unwrap().verify(first_data));
        assert!(manager.piece_hash(1).unwrap().verify(second_data));
        assert!(!manager.piece_hash(1).unwrap().verify(b"HELLO"));
    }

    #[test]
    fn hybrid_pieces_require_both_hash_families() {
        let mut first_piece = b"abc".to_vec();
        first_piece.resize(16 * 1024, 0);
        let second_piece = b"hello".to_vec();
        let meta = TorrentMeta {
            announce: None,
            announce_list: Vec::new(),
            url_list: Vec::new(),
            httpseeds: Vec::new(),
            info_hash: [0; 20],
            info_hash_v2: Some([0; 32]),
            piece_layers: Vec::new(),
            meta_version: 3,
            info: InfoDict {
                name: b"hybrid".to_vec(),
                piece_length: 16 * 1024,
                pieces: vec![sha1::sha1(&first_piece), sha1::sha1(&second_piece)],
                length: None,
                files: vec![
                    FileInfo {
                        length: 3,
                        path: vec![b"a".to_vec()],
                        attr: Vec::new(),
                    },
                    FileInfo {
                        length: 16 * 1024 - 3,
                        path: vec![b".pad".to_vec(), b"16381".to_vec()],
                        attr: b"p".to_vec(),
                    },
                    FileInfo {
                        length: 5,
                        path: vec![b"b".to_vec()],
                        attr: Vec::new(),
                    },
                ],
                private: false,
                file_tree: vec![
                    FileTreeEntry {
                        path: vec![b"a".to_vec()],
                        length: 3,
                        pieces_root: Some(sha256::sha256(b"abc")),
                    },
                    FileTreeEntry {
                        path: vec![b"b".to_vec()],
                        length: 5,
                        pieces_root: Some(sha256::sha256(b"hello")),
                    },
                ],
            },
        };

        let manager = PieceManager::new(&meta).unwrap();
        assert!(matches!(
            manager.piece_hash(0),
            Some(PieceHash::Hybrid { .. })
        ));
        assert!(manager.piece_hash(0).unwrap().verify(&first_piece));
        assert!(manager.piece_hash(1).unwrap().verify(&second_piece));

        let mut nonzero_padding = first_piece;
        *nonzero_padding.last_mut().unwrap() = 1;
        assert!(!manager.piece_hash(0).unwrap().verify(&nonzero_padding));
    }
}
