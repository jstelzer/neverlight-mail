# Installation & Sibling Setup Guide

This guide documents everything needed for the `just install` recipe and the development environment to work.

## 1. Sibling Directory Layout (Workspace Dependencies)

The `Cargo.toml` file in `neverlight-mail` and its dependent libraries rely on path-based sister dependencies located in the parent directory (`../`). 

For compilation to succeed, you must clone the following sibling repositories in the same directory containing `neverlight-mail`:

```bash
# Clone neverlight-mail's sister crates into the parent directory
git clone https://github.com/jstelzer/neverlight-mail-core.git ../neverlight-mail-core
git clone https://github.com/jstelzer/neverlight-mail-oauth.git ../neverlight-mail-oauth
git clone https://github.com/jstelzer/neverlight-mail-html-safe-md.git ../neverlight-mail-html-safe-md
```

**Expected Directory Structure:**
```text
Projektit/
├── neverlight-mail/            <- (You are here)
├── neverlight-mail-core/
├── neverlight-mail-oauth/
└── neverlight-mail-html-safe-md/
```

---

## 2. Rust Toolchain Requirements

This project utilizes advanced features and nightly compiler options:
* **Toolchain**: Rust `nightly` is required.
* To install or configure the nightly compiler for this directory, run:
  ```bash
  rustup override set nightly
  ```

---

## 3. System Development Libraries (libcosmic / wgpu dependencies)

Building COSMIC desktop GUI applications via `libcosmic` requires Wayland, X11, graphics drivers, and secret storage API libraries.

### Debian / Ubuntu / Pop!_OS
Install the required packages using `apt`:
```bash
sudo apt install -y \
    build-essential \
    pkg-config \
    libwayland-dev \
    libxkbcommon-dev \
    libegl1-mesa-dev \
    libfontconfig1-dev \
    libasound2-dev \
    libdbus-1-dev \
    libsecret-1-dev
```

### Fedora / Red Hat
Install the packages using `dnf`:
```bash
sudo dnf install -y \
    gcc \
    gcc-c++ \
    pkgconf-pkg-config \
    wayland-devel \
    libxkbcommon-devel \
    mesa-libEGL-devel \
    fontconfig-devel \
    alsa-lib-devel \
    dbus-devel \
    libsecret-devel
```

---

## 4. Desktop Integration Prerequisites (Gtk & Desktop DB)

The `just install` recipe copies the release binary, `.desktop` files, `.metainfo.xml` files, and icons into the local user space (`$PREFIX`, defaulting to `$HOME/.local`). It runs cache updates to ensure the application appears in your launcher menu.

Make sure the following GTK helper utilities are installed:
* `gtk-update-icon-cache`: Registers and refreshes the icons in the launcher.
* `update-desktop-database`: Refreshes applications list.

They are typically installed by default on modern GTK-based Linux desktop environments, or can be installed via:
* **Debian/Ubuntu/Pop!_OS**: `sudo apt install -y desktop-file-utils gtk-update-icon-cache`
* **Fedora**: `sudo dnf install -y desktop-file-utils gtk-update-icon-cache`

---

## 5. Using the `just` Command Runner

The compilation and installation uses the `just` command runner. If you do not have it, you can install it via:
* **Cargo**: `cargo install just`
* **Debian/Ubuntu/Pop!_OS**: `sudo apt install -y just`
* **Fedora**: `sudo dnf install -y just`

### Running the Installation

By default, files are installed to your user local folder (`$HOME/.local`).

```bash
# Build the optimized release binary, copy resources, and update launcher database
just install
```

To install into a custom prefix (e.g. `/usr/local`):
```bash
PREFIX=/usr/local just install
```
