//! Transaction ID and per-variable lock state for the Manager.
//!
//! Simplified implementation for issue #19. The existing transaction.rs
//! and lock.rs provide the full actor-based infrastructure for future use.

use std::collections::{HashMap, HashSet};
use std::time::SystemTime;
use crate::runtime::ast::Value;

/// A globally unique transaction identifier.
/// Older timestamp = higher priority (for future wait-die implementation).
/// Higher iteration = higher priority among retries of the same transaction.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TxnId {
    /// Wall-clock creation time, used for priority ordering.
    pub timestamp: SystemTime,
    /// Incremented on retry so retried transactions gain priority.
    pub iteration: u32,
}

impl TxnId {
    pub fn new() -> Self {
        TxnId {
            timestamp: SystemTime::now(),
            iteration: 0,
        }
    }

    /// Return a new TxnId with the same timestamp but higher iteration,
    /// for use when retrying an aborted transaction.
    pub fn retry(&self) -> Self {
        TxnId {
            timestamp: self.timestamp,
            iteration: self.iteration + 1,
        }
    }
}

impl PartialOrd for TxnId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TxnId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Older timestamp = smaller = higher priority
        // Higher iteration = higher priority among retries
        self.timestamp
            .cmp(&other.timestamp)
            .then(other.iteration.cmp(&self.iteration))
    }
}

/// Per-variable lock state used by the Manager.
/// Multiple readers are allowed simultaneously; writers are exclusive.
#[derive(Debug, Clone)]
pub enum VarLock {
    Unlocked,
    ReadLocked(HashSet<TxnId>),
    WriteLocked(TxnId),
}

impl VarLock {
    pub fn new() -> Self {
        VarLock::Unlocked
    }

    /// Try to acquire a read lock. Succeeds unless write-locked.
    pub fn try_read(&mut self, txn_id: &TxnId) -> bool {
        match self {
            VarLock::Unlocked => {
                let mut set = HashSet::new();
                set.insert(txn_id.clone());
                *self = VarLock::ReadLocked(set);
                true
            }
            VarLock::ReadLocked(set) => {
                set.insert(txn_id.clone());
                true
            }
            VarLock::WriteLocked(_) => false,
        }
    }

    /// Try to acquire an exclusive write lock. Fails if any lock is held.
    pub fn try_write(&mut self, txn_id: &TxnId) -> bool {
        match self {
            VarLock::Unlocked => {
                *self = VarLock::WriteLocked(txn_id.clone());
                true
            }
            _ => false,
        }
    }

    /// Release a read lock held by txn_id.
    pub fn release_read(&mut self, txn_id: &TxnId) {
        if let VarLock::ReadLocked(set) = self {
            set.remove(txn_id);
            if set.is_empty() {
                *self = VarLock::Unlocked;
            }
        }
    }

    /// Release the write lock if currently held by txn_id.
    pub fn release_write(&mut self, txn_id: &TxnId) {
        if matches!(self, VarLock::WriteLocked(tid) if tid == txn_id) {
            *self = VarLock::Unlocked;
        }
    }

    /// Upgrade a read lock held solely by txn_id to a write lock.
    /// Needed for read-then-write patterns (e.g. x = x + 1).
    /// Returns true if upgrade succeeded or var is already write-locked by txn_id.
    pub fn upgrade_to_write(&mut self, txn_id: &TxnId) -> bool {
        match self {
            VarLock::ReadLocked(set) if set.len() == 1 && set.contains(txn_id) => {
                *self = VarLock::WriteLocked(txn_id.clone());
                true
            }
            VarLock::WriteLocked(tid) if tid == txn_id => true,
            _ => false,
        }
    }

    /// Release any lock (read or write) held by txn_id.
    pub fn release(&mut self, txn_id: &TxnId) {
        match self {
            VarLock::ReadLocked(_) => self.release_read(txn_id),
            VarLock::WriteLocked(_) => self.release_write(txn_id),
            VarLock::Unlocked => {}
        }
    }
}

/// Composite state for a single variable, consolidating value, lock, and
/// transaction history into one structure instead of three separate maps.
#[derive(Debug, Clone)]
pub struct VarState {
    /// Current value of the variable.
    pub value: crate::runtime::ast::Value,
    /// Lock state for 2-phase locking.
    pub lock: VarLock,
    /// Most recent transaction to write this variable.
    pub latest_write_txn: Option<TxnId>,
}

impl VarState {
    pub fn new(value: crate::runtime::ast::Value) -> Self {
        VarState {
            value,
            lock: VarLock::new(),
            latest_write_txn: None,
        }
    }
}

/// Per-transaction state, owned by the code executing a transaction and passed
/// around during execution rather than stored on the Manager. A single Manager
/// may eventually run multiple transactions concurrently, so transaction state
/// must not live on the Manager.
#[derive(Debug)]
pub struct Transaction {
    /// Globally unique transaction identifier.
    pub id: TxnId,
    /// Variables this transaction currently holds a lock on.
    pub locked: HashSet<String>,
    /// Values already read in this transaction (avoids re-fetching, including
    /// redundant network round-trips for remote reads).
    pub read_cache: HashMap<String, Value>,
    /// Values written by this transaction, buffered and applied to the
    /// service only on successful commit (so a failed transaction leaves no
    /// partial writes).
    pub written: HashMap<String, Value>,
}

impl Transaction {
    pub fn new(id: TxnId) -> Self {
        Transaction {
            id,
            locked: HashSet::new(),
            read_cache: HashMap::new(),
            written: HashMap::new(),
        }
    }
}
