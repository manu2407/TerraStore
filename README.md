# 🏪 Terra Store

> A native TUI package manager for Arch Linux with fuzzy search and AUR support

![Rust](https://img.shields.io/badge/Rust-000000?style=flat-square&logo=rust&logoColor=white)
![Arch Linux](https://img.shields.io/badge/Arch_Linux-1793D1?style=flat-square&logo=arch-linux&logoColor=white)
![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)

---

## Why TerraStore Exists

This project began life inside my dotfiles, with a very simple goal: to create the **cleanest, least cluttered, and easiest way to manage an Arch Linux system**. I wanted everything centralized, reproducible, and easy to reason about—no scattered scripts, no fragile manual steps, no unnecessary system pollution.

That idea took time to mature.

### The Journey

The first version was written in **plain shell (sh)**. It worked, but it was rough.

Later, I rewrote it in **Lua**, aiming for better structure and extensibility. While Lua gave me flexibility, that version turned out to be overkill for what I needed at the time. I eventually moved back to shell and stayed there for a long stretch, refining the system steadily until around version 2.5.

At **version 2.7**, I made a deliberate decision to rewrite the core in **Rust**. The early Rust versions (2.7 → 2.9) were honestly not great—messy, slower than shell, and poorly optimized. At one point, I seriously considered abandoning Rust altogether and returning to shell.

Instead, I paused and realized something important: **this wasn't Rust's fault—it was my code.**

So I rewrote it again. Carefully. Thoughtfully. I focused on cleaner abstractions, better modularity, and performance-aware design.

### The Result

The result surprised me.

The current version is:

- ⚡ **Faster** than the shell implementation
- 📦 **More modular** and extensible
- ✨ **Cleaner**, more expressive, and easier to maintain
- 💎 Frankly, just more **satisfying** to read and work on

Along the way, this project became my real introduction to Rust. It taught me how the language actually thinks—ownership, structure, performance trade-offs, and discipline. For a first serious Rust project, it has been an intense but rewarding learning experience.

### Why Separation?

At this point, TerraStore outgrew its original role.

It no longer felt like "just a dotfiles helper." It became a **system-level tool**—something I want to use globally across my machines as a proper package. That's why I decided to separate it into its own repository and treat it as a standalone project.

From now on:
- TerraStore will live **independently**
- It will be used **globally** on my systems
- Dotfiles will **consume** it, not contain it

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

## ⚠️ Security & Disclaimer

I want to be clear about one thing: **I am still new to Rust**, and this is my first serious program in the language.

While the tool works well for me, I don't yet have deep expertise in secure systems programming, and there may be security issues or edge cases I'm unaware of. If you spot anything concerning—unsafe patterns, vulnerabilities, or design flaws—please let me know.

> **I wouldn't recommend blindly deploying this as a critical system package unless you've reviewed it yourself.** If you believe it's safe enough and useful for your setup, feel free to use it—but do so consciously.

Feedback, reviews, stars, or suggestions are always welcome.

---

## 📜 License

MIT License - Feel free to use and modify!

---

## 🙏 Credits

Originally developed as part of [TerraFlow-Dotfiles](https://github.com/manu2407/TerraFlow-Dotfiles).

---

<p align="center">
  Made with ❤️ for the Arch Linux community
  <br>
  <em>— Manu, signing off.</em>
</p>
