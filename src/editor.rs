type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write, BufReader};
use std::path::PathBuf;

/// Files above this threshold (10 MB) are opened in large-file mode.
/// In large-file mode, undo/redo is disabled to avoid cloning huge strings.
pub const LARGE_FILE_THRESHOLD: u64 = 10 * 1024 * 1024;

/// Maximum total bytes the undo stack may hold before old entries are evicted.
const UNDO_STACK_MAX_BYTES: usize = 50 * 1024 * 1024; // 50 MB

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LineEnding {
    Crlf,
    Lf,
    Cr,
}

impl LineEnding {
    pub fn as_str(&self) -> &'static str {
        match self {
            LineEnding::Crlf => "\r\n",
            LineEnding::Lf => "\n",
            LineEnding::Cr => "\r",
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            LineEnding::Crlf => "Windows (CRLF)",
            LineEnding::Lf => "Unix (LF)",
            LineEnding::Cr => "Mac (CR)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabId(pub usize);

static TAB_ID_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

pub fn next_tab_id() -> TabId {
    TabId(TAB_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
}

pub fn ensure_tab_id_at_least(id: usize) {
    TAB_ID_COUNTER.fetch_max(id + 1, std::sync::atomic::Ordering::Relaxed);
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum Encoding {
    #[default]
    Utf8,
    Windows1252,
    Utf16Le,
    Utf16Be,
}

impl Encoding {
    pub fn name(&self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Windows1252 => "Windows-1252",
            Encoding::Utf16Le => "UTF-16LE",
            Encoding::Utf16Be => "UTF-16BE",
        }
    }

    pub fn to_encoding(&self) -> &'static encoding_rs::Encoding {
        match self {
            Encoding::Utf8 => encoding_rs::UTF_8,
            Encoding::Windows1252 => encoding_rs::WINDOWS_1252,
            Encoding::Utf16Le => encoding_rs::UTF_16LE,
            Encoding::Utf16Be => encoding_rs::UTF_16BE,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditorTab {
    pub id: TabId,
    pub title: String,
    pub content: String,
    pub path: Option<PathBuf>,
    pub is_dirty: bool,
    #[serde(default)]
    pub undo_stack: Vec<String>,
    #[serde(default)]
    pub redo_stack: Vec<String>,
    #[serde(default)]
    pub line_ending: LineEnding,
    #[serde(default)]
    pub encoding: Encoding,
    #[serde(default)]
    pub scroll_to_cursor: bool,
    #[serde(default)]
    pub center_cursor: bool,
    #[serde(default)]
    pub cursor_range: Option<(usize, usize)>,
    /// When true, undo/redo and per-frame content cloning are disabled.
    #[serde(default)]
    pub large_file: bool,
    /// Original file size in bytes (used for UI hints).
    #[serde(default)]
    pub file_size: u64,
    /// Cached line offsets for performance
    #[serde(skip)]
    pub line_offsets: Vec<usize>,
    #[serde(skip)]
    pub line_count: usize,
    /// Cached character count for status bar
    #[serde(skip)]
    pub char_count: usize,
    /// Snapshot of content before the current edit, used for undo without per-frame cloning.
    #[serde(skip)]
    pub undo_snapshot: String,
}

impl Default for LineEnding {
    fn default() -> Self {
        #[cfg(target_os = "windows")]
        return LineEnding::Crlf;
        #[cfg(not(target_os = "windows"))]
        return LineEnding::Lf;
    }
}

impl Default for EditorTab {
    fn default() -> Self {
        Self {
            id: next_tab_id(),
            title: "Untitled".to_string(),
            content: String::new(),
            path: None,
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            #[cfg(target_os = "windows")]
            line_ending: LineEnding::Crlf,
            #[cfg(not(target_os = "windows"))]
            line_ending: LineEnding::Lf,
            encoding: Encoding::Utf8,
            scroll_to_cursor: false,
            center_cursor: false,
            cursor_range: Some((0, 0)),
            large_file: false,
            file_size: 0,
            line_offsets: vec![0],
            line_count: 1,
            char_count: 0,
            undo_snapshot: String::new(),
        }
    }
}

impl EditorTab {
    pub fn new(path: Option<PathBuf>, content: String) -> Self {
        let title = path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string();

        let size = content.len() as u64;
        let is_large = size >= LARGE_FILE_THRESHOLD;
        let line_offsets = Self::calculate_line_offsets(&content);
        let line_count = content.lines().count().max(1);
        let char_count = content.chars().count();
        let undo_snapshot = if is_large { String::new() } else { content.clone() };
        
        Self {
            id: next_tab_id(),
            title,
            content,
            path,
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            #[cfg(target_os = "windows")]
            line_ending: LineEnding::Crlf,
            #[cfg(not(target_os = "windows"))]
            line_ending: LineEnding::Lf,
            encoding: Encoding::Utf8,
            scroll_to_cursor: false,
            center_cursor: false,
            cursor_range: Some((0, 0)),
            large_file: is_large,
            file_size: size,
            line_offsets,
            line_count,
            char_count,
            undo_snapshot,
        }
    }

    pub fn from_file(path: PathBuf) -> Result<Self> {
        // Get file size first to pre-allocate and detect large files
        let metadata = fs::metadata(&path)?;
        let file_size = metadata.len();
        let is_large = file_size >= LARGE_FILE_THRESHOLD;

        let file = fs::File::open(&path)?;
        let mut reader = BufReader::with_capacity(
            if is_large { 1024 * 1024 } else { 64 * 1024 },
            file,
        );
        let mut bytes = Vec::new();
        if is_large {
            use std::io::Read; // for the `take` trait method if not in scope
            // Prevent hanging and insane memory usage by hard-capping read bytes.
            let mut take_reader = reader.take(LARGE_FILE_THRESHOLD);
            take_reader.read_to_end(&mut bytes)?;
        } else {
            bytes.reserve_exact(file_size as usize);
            reader.read_to_end(&mut bytes)?;
        }

        // Try to detect encoding or fallback to UTF-8
        let (content, encoding, _had_errors) = if bytes.starts_with(b"\xFF\xFE") {
            let (res, _enc, had_errors) = encoding_rs::UTF_16LE.decode(&bytes[2..]);
            (res.into_owned(), Encoding::Utf16Le, had_errors)
        } else if bytes.starts_with(b"\xFE\xFF") {
            let (res, _enc, had_errors) = encoding_rs::UTF_16BE.decode(&bytes[2..]);
            (res.into_owned(), Encoding::Utf16Be, had_errors)
        } else {
            // Try UTF-8 first
            let (res, _enc, had_errors) = encoding_rs::UTF_8.decode(&bytes);
            if !had_errors {
                (res.into_owned(), Encoding::Utf8, false)
            } else {
                // If UTF-8 fails, try Windows-1252 as a common fallback
                let (res, _enc, had_errors) = encoding_rs::WINDOWS_1252.decode(&bytes);
                (res.into_owned(), Encoding::Windows1252, had_errors)
            }
        };

        // Drop the raw byte buffer early to free memory before we allocate more
        drop(bytes);

        let mut content = content;

        // For large files, only sample the first 64KB (at a valid char boundary) for line ending detection
        let sample = if is_large {
            let max_bytes = content.len().min(64 * 1024);
            let safe_len = content.floor_char_boundary(max_bytes);
            &content[..safe_len]
        } else {
            &content
        };

        // Detect line ending
        let line_ending = if sample.contains("\r\n") {
            LineEnding::Crlf
        } else if sample.contains('\r') {
            LineEnding::Cr
        } else {
            LineEnding::Lf
        };

        // Normalize to LF for editing
        if line_ending == LineEnding::Crlf {
            content.retain(|c| c != '\r');
        } else if line_ending == LineEnding::Cr {
            unsafe {
                let bytes = content.as_mut_vec();
                for b in bytes.iter_mut() {
                    if *b == b'\r' {
                        *b = b'\n';
                    }
                }
            }
        }

        let mut tab = Self::new(Some(path), content);
        tab.line_ending = line_ending;
        tab.encoding = encoding;
        tab.large_file = is_large;
        tab.file_size = file_size;

        if is_large {
            tab.content.push_str("\n\n... [File truncated: Cannot fully load files over 10MB in memory preview] ...");
        }

        Ok(tab)
    }

    pub fn save(&mut self) -> Result<()> {
        if self.large_file {
            return Err("File is too large and was loaded in truncated preview mode. Saving is disabled to prevent data loss.".into());
        }

        if let Some(path) = &self.path {
            let mut file = fs::File::create(path)?;

            // Convert LF to target line ending
            let content_to_save = if self.line_ending == LineEnding::Lf {
                std::borrow::Cow::Borrowed(&self.content)
            } else {
                std::borrow::Cow::Owned(self.content.replace('\n', self.line_ending.as_str()))
            };

            // Encode content to target encoding
            let (bytes, _enc, _had_errors) = self.encoding.to_encoding().encode(&content_to_save);

            // Add BOM if needed for UTF-16
            if self.encoding == Encoding::Utf16Le {
                file.write_all(b"\xFF\xFE")?;
            } else if self.encoding == Encoding::Utf16Be {
                file.write_all(b"\xFE\xFF")?;
            }

            file.write_all(&bytes)?;
            self.is_dirty = false;
            Ok(())
        } else {
            Err("No path set for file".into())
        }
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string();
        self.path = Some(path);
    }

    pub fn push_undo(&mut self, content: String) {
        // In large-file mode, undo is disabled to prevent huge memory usage
        if self.large_file {
            return;
        }
        self.undo_stack.push(content);
        self.redo_stack.clear();

        // Evict oldest entries if the stack exceeds the memory budget
        let mut total_bytes: usize = self.undo_stack.iter().map(|s| s.len()).sum();
        while total_bytes > UNDO_STACK_MAX_BYTES && !self.undo_stack.is_empty() {
            total_bytes -= self.undo_stack[0].len();
            self.undo_stack.remove(0);
        }

        // Also cap the number of entries as a secondary guard
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.content.clone());
            self.content = prev;
            self.is_dirty = true;
            self.refresh_metadata();
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.content.clone());
            self.content = next;
            self.is_dirty = true;
            self.refresh_metadata();
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn calculate_line_offsets(content: &str) -> Vec<usize> {
        let mut offsets = vec![0];
        for (i, b) in content.as_bytes().iter().enumerate() {
            if *b == b'\n' {
                offsets.push(i + 1);
            }
        }
        offsets
    }

    pub fn refresh_metadata(&mut self) {
        self.line_offsets = Self::calculate_line_offsets(&self.content);
        self.line_count = self.content.lines().count().max(1) + if self.content.ends_with('\n') { 1 } else { 0 };
        self.char_count = self.content.chars().count();
    }
}
