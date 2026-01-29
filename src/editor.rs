use std::path::PathBuf;
use std::fs;
use std::io::Write;
use anyhow::{Result, Context};

#[derive(Clone, Debug)]
pub struct EditorTab {
    pub id: uuid::Uuid,
    pub title: String,
    pub content: String,
    pub path: Option<PathBuf>,
    pub is_dirty: bool,
    // We can add undo/redo stack here later
}

impl Default for EditorTab {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            title: "Untitled".to_string(),
            content: String::new(),
            path: None,
            is_dirty: false,
        }
    }
}

impl EditorTab {
    pub fn new(path: Option<PathBuf>, content: String) -> Self {
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
        }
    }

    pub fn from_file(path: PathBuf) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read file: {:?}", path))?;
        Ok(Self::new(Some(path), content))
    }

    pub fn save(&mut self) -> Result<()> {
        if let Some(path) = &self.path {
            let mut file = fs::File::create(path)?;
            file.write_all(self.content.as_bytes())?;
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
}
