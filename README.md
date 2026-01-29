# Notos Text Editor

A lightweight, modern text editor built with Rust and egui, inspired by the simplicity of Notepad but enhanced with modern features.

## Features

- **Tabbed Interface**: Open and manage multiple files simultaneously.
- **Dynamic Zoom**: Independent editor zoom (Ctrl + Scroll) that doesn't affect the UI.
- **Line Numbers**: Accurate line numbering with support for word wrap (wrapped lines show blank spaces).
- **Dark Mode**: Beautiful "bit grey" dark theme for comfortable night-time editing.
- **Word Wrap**: Toggleable word wrapping with horizontal scrolling support when disabled.
- **Find & Replace**: Robust search and replace functionality.
- **Go To Line**: Quickly navigate to any line in your file.
- **Status Bar**: Real-time cursor position (Ln/Col), character count, encoding, and line ending indicators.
- **Plugin System**: Extensible architecture (includes an "About" plugin by default).
- **Cross-Platform**: Built with Rust for performance and portability.

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)

### Build from Source

```powershell
git clone https://github.com/appsjuragan/rust-notos
cd rust-notos
cargo build --release
```

The executable will be located in `target/release/rust-notos.exe`.

## Usage

- **Ctrl + N**: New Tab
- **Ctrl + O**: Open File
- **Ctrl + S**: Save File
- **Ctrl + Shift + S**: Save As
- **Ctrl + F**: Find
- **Ctrl + H**: Replace
- **Ctrl + G**: Go To Line
- **Ctrl + Scroll**: Zoom In/Out
- **F5**: Insert Time/Date

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Credits

Developed by [appsjuragan](https://github.com/appsjuragan).
