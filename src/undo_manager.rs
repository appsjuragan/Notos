//! Undo/Redo manager with TEMP-folder spilling and background I/O.
//!
//! Each tab may undo up to `MAX_UNDO_STEPS` (128) times.  Recent entries live
//! in memory (up to `MAX_MEMORY_ENTRIES` per stack); older entries are gzip-
//! compressed and written to `%TEMP%/notos_undo/`.  All disk I/O runs on a
//! dedicated background thread so the UI is never blocked.

use crate::editor::TabId;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// Maximum undo steps per tab (parameterised).
const MAX_UNDO_STEPS: usize = 128;

/// Entries kept in RAM per stack before spilling to disk.
const MAX_MEMORY_ENTRIES: usize = 16;

// ── Background I/O ──────────────────────────────────────────────────────────

enum BgTask {
    Spill { path: PathBuf, data: UndoEntry },
    DelFile { path: PathBuf },
    DelDir { path: PathBuf },
    Shutdown,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UndoEntry {
    pub content: String,
    pub cursor_pos: usize,
}

// ── Spilling stack ──────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct DiskRef {
    path: PathBuf,
}

/// A LIFO stack whose oldest entries can be transparently spilled to disk.
/// "back" = newest, "front" = oldest.
struct EntryStack {
    mem: VecDeque<UndoEntry>,
    disk: VecDeque<DiskRef>,
    seq: u64,
}

impl EntryStack {
    fn new() -> Self {
        Self { mem: VecDeque::new(), disk: VecDeque::new(), seq: 0 }
    }

    fn is_empty(&self) -> bool {
        self.mem.is_empty() && self.disk.is_empty()
    }

    fn len(&self) -> usize {
        self.mem.len() + self.disk.len()
    }

    /// Push newest entry.
    fn push(&mut self, entry: UndoEntry) {
        self.mem.push_back(entry);
    }

    /// Pop newest entry (memory first, then disk).
    fn pop(&mut self) -> Option<UndoEntry> {
        if let Some(e) = self.mem.pop_back() {
            return Some(e);
        }
        if let Some(r) = self.disk.pop_back() {
            return Self::load(&r.path);
        }
        None
    }

    /// Drop every entry, deleting disk files via the bg thread.
    fn clear(&mut self, tx: &mpsc::Sender<BgTask>) {
        self.mem.clear();
        for r in self.disk.drain(..) {
            let _ = tx.send(BgTask::DelFile { path: r.path });
        }
    }

    /// Move oldest in-memory entries to disk when memory count exceeds cap.
    fn spill(&mut self, cap: usize, dir: &Path, prefix: &str, tx: &mpsc::Sender<BgTask>) {
        while self.mem.len() > cap {
            if let Some(old) = self.mem.pop_front() {
                let p = dir.join(format!("{}_{}.gz", prefix, self.seq));
                self.seq += 1;
                self.disk.push_back(DiskRef { path: p.clone() });
                let _ = tx.send(BgTask::Spill { path: p, data: old });
            }
        }
    }

    /// Evict oldest entries (disk first) to stay within a total cap.
    fn evict(&mut self, cap: usize, tx: &mpsc::Sender<BgTask>) {
        while self.len() > cap {
            if let Some(r) = self.disk.pop_front() {
                let _ = tx.send(BgTask::DelFile { path: r.path });
            } else if self.mem.pop_front().is_some() {
                // dropped
            } else {
                break;
            }
        }
    }

    fn load(path: &Path) -> Option<UndoEntry> {
        let data = std::fs::read(path).ok()?;
        let mut dec = GzDecoder::new(&data[..]);
        let mut json = String::new();
        dec.read_to_string(&mut json).ok()?;
        let entry: UndoEntry = serde_json::from_str(&json).ok()?;
        // Note: we don't remove the file here anymore if we want to support persistence 
        // across pop/push cycles during a save, but actually pop() is fine to remove 
        // because once it's in memory, it's in memory.
        let _ = std::fs::remove_file(path);
        Some(entry)
    }

    /// Spill everything to disk for persistence.
    fn spill_all(&mut self, dir: &Path, prefix: &str, tx: &mpsc::Sender<BgTask>) {
        self.spill(0, dir, prefix, tx);
    }
}

// ── Per-tab state ───────────────────────────────────────────────────────────

struct TabState {
    undo: EntryStack,
    redo: EntryStack,
}

impl TabState {
    fn new() -> Self {
        Self { undo: EntryStack::new(), redo: EntryStack::new() }
    }
}

// ── Persistence Structs ───────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct PersistentEntryStack {
    pub disk: Vec<PathBuf>,
    pub seq: u64,
}

#[derive(Serialize, Deserialize)]
pub struct PersistentTabState {
    pub undo: PersistentEntryStack,
    pub redo: PersistentEntryStack,
}

#[derive(Serialize, Deserialize, Default)]
pub struct PersistentUndoState {
    pub tabs: HashMap<usize, PersistentTabState>,
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Manages per-tab undo/redo with disk spilling and background I/O.
pub struct UndoManager {
    tabs: HashMap<usize, TabState>,
    dir: PathBuf,
    tx: mpsc::Sender<BgTask>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl UndoManager {
    pub fn new(state: Option<PersistentUndoState>) -> Self {
        let dir = std::env::temp_dir().join("notos_undo");
        // Don't remove_dir_all anymore, just ensure it exists
        let _ = std::fs::create_dir_all(&dir);

        let (tx, rx) = mpsc::channel::<BgTask>();
        let handle = std::thread::Builder::new()
            .name("notos-undo-io".into())
            .spawn(move || Self::worker(rx))
            .expect("spawn undo-io thread");

        let mut mgr = Self { tabs: HashMap::new(), dir: dir.clone(), tx, handle: Some(handle) };

        if let Some(s) = state {
            for (id, ts) in s.tabs {
                mgr.tabs.insert(id, TabState {
                    undo: EntryStack {
                        mem: VecDeque::new(),
                        disk: ts.undo.disk.into_iter().map(|p| DiskRef { path: p }).collect(),
                        seq: ts.undo.seq,
                    },
                    redo: EntryStack {
                        mem: VecDeque::new(),
                        disk: ts.redo.disk.into_iter().map(|p| DiskRef { path: p }).collect(),
                        seq: ts.redo.seq,
                    },
                });
            }
        }

        mgr
    }

    // ── queries ─────────────────────────────────────────────────────────

    pub fn can_undo(&self, id: TabId) -> bool {
        self.tabs.get(&id.0).map_or(false, |s| !s.undo.is_empty())
    }

    pub fn can_redo(&self, id: TabId) -> bool {
        self.tabs.get(&id.0).map_or(false, |s| !s.redo.is_empty())
    }

    // ── mutations ───────────────────────────────────────────────────────

    /// Record an undo snapshot.  Clears the redo stack (new edit).
    pub fn push_undo(&mut self, id: TabId, content: String, cursor_pos: usize, large: bool) {
        if large { return; }
        let s = self.tabs.entry(id.0).or_insert_with(TabState::new);
        s.redo.clear(&self.tx);
        s.undo.push(UndoEntry { content, cursor_pos });
        let td = self.dir.join(format!("t{}", id.0));
        s.undo.spill(MAX_MEMORY_ENTRIES, &td, "u", &self.tx);
        s.undo.evict(MAX_UNDO_STEPS, &self.tx);
    }

    /// Undo: returns entry to restore, or `None`.
    pub fn undo(&mut self, id: TabId, current: String, current_cursor: usize) -> Option<UndoEntry> {
        let s = self.tabs.get_mut(&id.0)?;
        let prev = s.undo.pop()?;
        s.redo.push(UndoEntry { content: current, cursor_pos: current_cursor });
        let td = self.dir.join(format!("t{}", id.0));
        s.redo.spill(MAX_MEMORY_ENTRIES, &td, "r", &self.tx);
        Some(prev)
    }

    /// Redo: returns entry to restore, or `None`.
    pub fn redo(&mut self, id: TabId, current: String, current_cursor: usize) -> Option<UndoEntry> {
        let s = self.tabs.get_mut(&id.0)?;
        let next = s.redo.pop()?;
        s.undo.push(UndoEntry { content: current, cursor_pos: current_cursor });
        let td = self.dir.join(format!("t{}", id.0));
        s.undo.spill(MAX_MEMORY_ENTRIES, &td, "u", &self.tx);
        s.undo.evict(MAX_UNDO_STEPS, &self.tx);
        Some(next)
    }

    /// Export current state for persistence. Spills all in-memory entries to disk first.
    pub fn export_persistent_state(&mut self) -> PersistentUndoState {
        let mut state = PersistentUndoState::default();
        for (id, ts) in self.tabs.iter_mut() {
            let td = self.dir.join(format!("t{}", id));
            ts.undo.spill_all(&td, "u", &self.tx);
            ts.redo.spill_all(&td, "r", &self.tx);

            state.tabs.insert(*id, PersistentTabState {
                undo: PersistentEntryStack {
                    disk: ts.undo.disk.iter().map(|r| r.path.clone()).collect(),
                    seq: ts.undo.seq,
                },
                redo: PersistentEntryStack {
                    disk: ts.redo.disk.iter().map(|r| r.path.clone()).collect(),
                    seq: ts.redo.seq,
                },
            });
        }
        state
    }

    /// Clean up all state for a closed tab.
    pub fn remove_tab(&mut self, id: TabId) {
        if let Some(mut st) = self.tabs.remove(&id.0) {
            st.undo.clear(&self.tx);
            st.redo.clear(&self.tx);
            let _ = self.tx.send(BgTask::DelDir {
                path: self.dir.join(format!("t{}", id.0)),
            });
        }
    }

    /// Clean up everything (app exit). 
    /// Note: We don't delete the directory here anymore to support persistence.
    pub fn cleanup_all(&mut self) {
        self.tabs.clear();
    }

    // ── background worker ───────────────────────────────────────────────

    fn worker(rx: mpsc::Receiver<BgTask>) {
        for task in rx {
            match task {
                BgTask::Spill { path, data } => {
                    if let Some(p) = path.parent() {
                        let _ = std::fs::create_dir_all(p);
                    }
                    match std::fs::File::create(&path) {
                        Ok(f) => {
                            let mut enc = GzEncoder::new(f, Compression::fast());
                            if let Ok(json) = serde_json::to_string(&data) {
                                let _ = enc.write_all(json.as_bytes());
                                let _ = enc.finish();
                            }
                        }
                        Err(e) => log::error!("undo spill {:?}: {}", path, e),
                    }
                }
                BgTask::DelFile { path } => { let _ = std::fs::remove_file(path); }
                BgTask::DelDir  { path } => { let _ = std::fs::remove_dir_all(path); }
                BgTask::Shutdown => break,
            }
        }
    }
}

impl Drop for UndoManager {
    fn drop(&mut self) {
        self.cleanup_all();
        let _ = self.tx.send(BgTask::Shutdown);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}
