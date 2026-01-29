use std::path::PathBuf;
use std::fs;
use std::io::Write;
use anyhow::{Result, Context};

use encoding_rs::Encoding;

#[derive(Clone, Copy, Debug, PartialEq)]
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

#[derive(Clone, Debug)]
pub struct EditorTab {
    pub id: uuid::Uuid,
    pub title: String,
    pub content: String,
    pub path: Option<PathBuf>,
    pub is_dirty: bool,
    pub undo_stack: Vec<String>,
    pub redo_stack: Vec<String>,
    pub encoding: &'static Encoding,
    pub line_ending: LineEnding,
}

impl Default for EditorTab {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            title: "Untitled".to_string(),
            content: String::new(),
            path: None,
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            encoding: encoding_rs::UTF_8,
            #[cfg(target_os = "windows")]
            line_ending: LineEnding::Crlf,
            #[cfg(not(target_os = "windows"))]
            line_ending: LineEnding::Lf,
        }
    }
}

impl EditorTab {
    pub fn new(path: Option<PathBuf>, content: String, encoding: &'static Encoding) -> Self {
        let title = path.as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string();

        Self {
            id: uuid::Uuid::new_v4(),
            title,
            content,
            path,
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            encoding,
            #[cfg(target_os = "windows")]
            line_ending: LineEnding::Crlf,
            #[cfg(not(target_os = "windows"))]
            line_ending: LineEnding::Lf,
        }
    }

    pub fn from_file(path: PathBuf) -> Result<Self> {
        let bytes = fs::read(&path)
            .with_context(|| format!("Failed to read file: {:?}", path))?;
        
        let (cow, _, _) = encoding_rs::UTF_8.decode(&bytes);
        let mut content = cow.into_owned();
        
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
        
        let mut tab = Self::new(Some(path), content, encoding_rs::UTF_8);
        tab.line_ending = line_ending;
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

            let (cow, _, _) = self.encoding.encode(&content_to_save);
            file.write_all(&cow)?;
            self.is_dirty = false;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No path set for file"))
        }
    }

    pub fn set_content(&mut self, content: String) {
        if self.content != content {
            self.content = content;
            self.is_dirty = true;
        }
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.title = path.file_name()
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
}
