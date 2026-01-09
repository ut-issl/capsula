---
icon: material/download
---

# Installation

This guide will help you install Capsula on your system.

## Requirements

- **Rust toolchain** (version 1.90.0 or later)

!!! tip "Don't have Rust installed?"
    Visit [rustup.rs](https://rustup.rs/) to install Rust. It takes just a few minutes!

## Installing Capsula

### Method 1: Install from crates.io (Recommended)

This is the easiest way to install Capsula:

```bash
cargo install capsula-cli --locked
```

This downloads and compiles the latest stable version from Rust's package registry.

### Method 2: Install from GitHub

To install the latest development version:

```bash
cargo install --git https://github.com/ut-issl/capsula --locked capsula-cli
```

!!! warning "Development version"
    The GitHub version may have new features but could also be less stable than the released version.

## Verify Installation

After installation, verify that Capsula is working:

```bash
capsula --version
```

You should see output like:

```
capsula 0.9.0
```

Try the help command:

```bash
capsula --help
```

## Updating Capsula

To update Capsula to the latest version, run the same install command again:

```bash
cargo install capsula-cli --locked
```

Cargo will automatically download and install the new version.

## Troubleshooting

### "cargo: command not found"

Rust is not installed or not in your PATH. Install Rust from [rustup.rs](https://rustup.rs/) and restart your terminal.

### Compilation Errors

**Rust version too old:**

Check your Rust version:

```bash
rustc --version
```

Capsula requires Rust 1.90.0 or later. Update Rust:

```bash
rustup update
```

Then try installing again:

```bash
cargo install capsula-cli --locked
```

**Missing system libraries:**

If compilation fails with "linker" or "library not found" errors, you may need to install system development libraries. Read the error output carefully - it usually indicates which library is missing.

## Uninstalling Capsula

If you need to uninstall Capsula:

```bash
cargo uninstall capsula-cli
```

This removes the Capsula binary but doesn't delete any captured data in your `.capsula` directories.
