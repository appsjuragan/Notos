# Notos

**Notos** is a high-performance, lightweight text editor built with Rust and egui. It combines the minimalist philosophy of classic editors like Notepad with modern, developer-centric features like independent text zooming, smart line numbering, and a plugin-ready architecture.

---

## 🚀 Key Features

- **⚡ Extreme Binary Size Optimization**: Reached **~1.8MB** (from ~13MB) by removing embedded fonts and trimming heavy dependencies like `anyhow`, `uuid`, and `encoding_rs`.
- **🧩 Dynamic Plugin System**: Robust architecture using an external SDK (`notos_sdk`) to load `.dll`/`.so` plugins at runtime without recompiling the main app.
- **🎨 System Font Loader**: Dynamically loads fonts from the OS (e.g., Segoe UI, Consolas, Segoe UI Symbol/Emoji on Windows). This keeps the binary small while ensuring full UTF-8 icon support.
- **💾 Zero Data Loss**: Automatically saves your session (tabs, content, undo history, and selections) on close and restores it instantly upon reopening.
- **🖱️ Right-Click Context Menu**: Full context menu support for Undo, Redo, Cut, Copy, Paste, and Select All.
- **⚡ Blazing Fast Performance**: Built with Rust for a near-instant startup and smooth editing experience, even with large files.
- **📑 Tabbed Workflow**: Effortlessly manage multiple documents within a single, clean window.
- **🔍 Independent Editor Zoom**: Scale your text (Ctrl + Scroll) without affecting the UI scale.
- **🔢 Smart Line Numbering**: Accurate line tracking that understands word wrapping. Wrapped lines show blank spaces in the gutter, maintaining logical line alignment.
- **🌙 "Bit Grey" Dark Mode**: A custom-tuned dark theme designed to reduce eye strain.
- **↔️ Flexible Word Wrap**: Toggle wrapping on the fly.
- **📊 Comprehensive Status Bar**: Real-time tracking of cursor position (Ln/Col), character count, and line endings (CRLF/LF).

---

## ⌨️ Keyboard Shortcuts

| Action | Shortcut |
| :--- | :--- |
| **New Tab** | `Ctrl + N` |
| **Open File** | `Ctrl + O` |
| **Save File** | `Ctrl + S` |
| **Save As** | `Ctrl + Shift + S` |
| **Find** | `Ctrl + F` |
| **Replace** | `Ctrl + H` |
| **Go To Line** | `Ctrl + G` |
| **Zoom In/Out** | `Ctrl + Mouse Wheel` |
| **Insert Date/Time** | `F5` |

---

## 🛠️ Installation

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) (Stable toolchain)

### Build from Source
```bash
# Clone the repository
git clone https://github.com/appsjuragan/Notos

# Navigate to the project directory
cd Notos

# Build the release binary
cargo build --release
```
The compiled executable will be available in `target/release/`.

---

## 🤝 Contributing

Contributions are welcome! Whether it's reporting a bug, suggesting a feature, or submitting a pull request, your help makes Notos better for everyone.

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

---

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.

---

## ✨ Credits

Developed and maintained by [appsjuragan](https://github.com/appsjuragan).
