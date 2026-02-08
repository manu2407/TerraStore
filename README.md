# 🏪 Terra Store

> A native TUI package manager for Arch Linux with fuzzy search and AUR support

![Rust](https://img.shields.io/badge/Rust-000000?style=flat-square&logo=rust&logoColor=white)
![Arch Linux](https://img.shields.io/badge/Arch_Linux-1793D1?style=flat-square&logo=arch-linux&logoColor=white)
![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)

---

## ✨ Features

- 🔍 **Zero-Stress Search** - Instant fuzzy search across all packages
- 📦 **Multi-Source** - Official repos, AUR, and Flatpak support
- 🎨 **Beautiful TUI** - Gruvbox-themed terminal interface
- 📊 **Installation History** - Track what you've installed
- 🔄 **TerraFlow Integration** - Optional sync with dotfiles package lists

---

## 📥 Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/manu2407/TerraStore.git
cd TerraStore

# Build and install
cargo build --release
sudo install -Dm755 target/release/terra_store /usr/local/bin/terra-store
```

### From Cargo (when published)

```bash
cargo install terra_store
```

---

## 🚀 Usage

```bash
# Launch the TUI
terra-store

# Or run directly from target
./target/release/terra_store
```

### Keybindings

| Key | Action |
|-----|--------|
| `↑/↓` | Navigate packages |
| `Enter` | Install selected package |
| `/` | Focus search |
| `Tab` | Switch source (Official/AUR) |
| `q` | Quit |

---

## ⚙️ Configuration

### TerraFlow Integration (Optional)

Terra Store can sync with a dotfiles package list to show what's missing from your system.

Set the `TERRA_PACKAGES_DIR` environment variable to your packages directory:

```bash
export TERRA_PACKAGES_DIR="$HOME/.dotfiles/packages"
```

Or place package lists (`.txt` files) in one of these auto-detected locations:
- `~/.config/terra-store/packages`
- `~/TerraFlow-Dotfiles/packages`
- `~/.dotfiles/packages`
- `~/dotfiles/packages`

### Disabling TerraFlow

To build without TerraFlow integration:

```bash
cargo build --release --no-default-features
```

---

## 📁 Package List Format

Package lists are simple text files with one package per line:

```
# pacman_system.txt
base
base-devel
linux
linux-firmware
```

Files containing `aur` in the name are treated as AUR packages.

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    TERRA STORE TUI                          │
│   Search Bar  │  Package List  │  Details  │  Status Bar   │
└────────────────────────┬────────────────────────────────────┘
                         │
         ┌───────────────┼───────────────┐
         ▼               ▼               ▼
    ┌─────────┐    ┌──────────┐    ┌──────────┐
    │ Pacman  │    │   AUR    │    │ Flatpak  │
    │  Repo   │    │  (paru)  │    │          │
    └─────────┘    └──────────┘    └──────────┘
```

---

## 📦 Dependencies

- **Runtime**: `pacman`, `paru` (for AUR), `flatpak` (optional)
- **Build**: Rust 1.70+

---

## 🔒 Authentication

Terra Store uses `sudo` for package installation. It will prompt for your password when needed and cache credentials appropriately.

---

## 📜 License

MIT License - Feel free to use and modify!

---

## 🙏 Credits

Originally developed as part of [TerraFlow-Dotfiles](https://github.com/manu2407/TerraFlow-Dotfiles).

---

<p align="center">
  Made with ❤️ for the Arch Linux community
</p>
