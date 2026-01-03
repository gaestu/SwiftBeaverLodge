# SwiftBeaverLodge

🦫 **GUI Frontend for SwiftBeaver Forensic File Carver**

SwiftBeaverLodge is a modern desktop application for forensic file recovery, built with Tauri and Svelte. It provides a user-friendly interface for the powerful SwiftBeaver file carving engine.

## Features

- 📂 **Multi-format evidence support** - Raw images (.dd, .raw, .img), E01 format, and block devices
- 🔍 **Comprehensive file recovery** - Images, documents, archives, databases, media, and more
- 🔗 **String/URL/Email extraction** - Find text patterns in evidence
- 📊 **Real-time progress monitoring** - Live throughput, ETA, and statistics
- 🖼️ **Result browsing with thumbnails** - Preview recovered files
- 💾 **Offline/Airgapped operation** - No network required for forensic integrity
- 🚀 **GPU acceleration** - Optional OpenCL/CUDA support

## Prerequisites

### Linux System Dependencies

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev

# Fedora
sudo dnf install -y \
  webkit2gtk4.1-devel \
  gtk3-devel \
  libappindicator-gtk3-devel \
  librsvg2-devel \
  libsoup3-devel \
  javascriptcoregtk4.1-devel

# Arch Linux
sudo pacman -S --needed \
  webkit2gtk-4.1 \
  base-devel \
  curl \
  wget \
  openssl \
  appmenu-gtk-module \
  libappindicator-gtk3 \
  librsvg
```

### Additional Requirements

- **Rust** (rustup.rs recommended)
- **Node.js** 18+ and npm

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/SwiftBeaverLodge.git
cd SwiftBeaverLodge

# Install Node.js dependencies
npm install

# Build and run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Usage

1. **Select Evidence Source** - Choose a disk image, E01 file, or block device
2. **Configure Output** - Set the output directory for recovered files
3. **Choose File Types** - Select which file types to recover
4. **Configure Options** - Enable string scanning, GPU acceleration, etc.
5. **Start Scan** - Monitor progress in real-time
6. **Browse Results** - View recovered files and artefacts

## Project Structure

```
SwiftBeaverLodge/
├── src/                    # Svelte frontend
│   ├── lib/
│   │   ├── api/           # Tauri IPC wrappers
│   │   ├── components/    # UI components
│   │   ├── stores/        # State management
│   │   └── utils/         # Utilities
│   └── routes/            # SvelteKit routes
├── src-tauri/              # Rust backend
│   └── src/
│       ├── commands/      # Tauri commands
│       ├── scan/          # Scan management
│       └── types.rs       # Type definitions
└── docs/                   # Documentation
```

## Development

```bash
# Run development server
npm run tauri dev

# Type check
npm run check

# Build for production
npm run tauri build
```

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

## License

MIT

## Acknowledgments

- [SwiftBeaver](https://github.com/gaestu/SwiftBeaver) - The forensic file carving engine
- [Tauri](https://tauri.app) - Desktop application framework
- [Svelte](https://svelte.dev) - Frontend framework
