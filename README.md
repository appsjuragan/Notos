# Notos

**Notos** is a high-performance, lightweight text editor built with Rust and egui. It combines the minimalist philosophy of classic editors like Notepad with modern, developer-centric features like independent text zooming, smart line numbering, and a plugin-ready architecture.

---

## 🚀 Key Features

- **⚡ Blazing Fast Performance**: Built with Rust for a near-instant startup and smooth editing experience, even with large files.
- **📑 Tabbed Workflow**: Effortlessly manage multiple documents within a single, clean window.
- **🔍 Independent Editor Zoom**: Scale your text (Ctrl + Scroll) without affecting the UI scale—perfect for presentations or high-DPI displays.
- **🔤 Custom Font Selection**: Switch between monospace and proportional fonts, or load your own `.ttf`/`.otf` files for a personalized editing experience.
- **🖱️ Drag & Drop Support**: Quickly open files by dropping them anywhere into the editor window.
- **🔢 Smart Line Numbering**: Accurate line tracking that understands word wrapping. Wrapped lines show blank spaces in the gutter, maintaining logical line alignment.
- **🌙 "Bit Grey" Dark Mode**: A custom-tuned dark theme designed to reduce eye strain while maintaining high contrast for readability.
- **↔️ Flexible Word Wrap**: Toggle wrapping on the fly. When disabled, the editor provides smooth horizontal scrolling for long lines of code or data.
- **🛠️ Extensible Architecture**: Includes a modular plugin system (featuring a built-in "About" plugin) designed for future community enhancements.
- **📊 Comprehensive Status Bar**: Real-time tracking of cursor position (Ln/Col), character count, encoding (UTF-8/UTF-16), and line endings (CRLF/LF).

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
