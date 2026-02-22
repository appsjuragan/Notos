type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

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
        }
    }

    pub fn from_file(path: PathBuf) -> Result<Self> {
        let bytes = fs::read(&path)?;

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

        let mut content = content;

        // Detect line ending
        let line_ending = if content.contains("\r\n") {
            LineEnding::Crlf
        } else if content.contains('\r') {
            LineEnding::Cr
        } else {
            LineEnding::Lf
        };

        // Normalize to LF for editing
        if line_ending != LineEnding::Lf {
            content = content.replace("\r\n", "\n").replace('\r', "\n");
        }

        let mut tab = Self::new(Some(path), content);
        tab.line_ending = line_ending;
        tab.encoding = encoding;
        Ok(tab)
    }

    pub fn save(&mut self) -> Result<()> {
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
        self.undo_stack.push(content);
        self.redo_stack.clear();
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.content.clone());
            self.content = prev;
            self.is_dirty = true;
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.content.clone());
            self.content = next;
            self.is_dirty = true;
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}
