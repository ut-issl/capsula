# Installation

This guide will help you install Capsula on your system.

## Requirements

Before installing Capsula, make sure you have:

- **Rust toolchain** (version 1.90.0 or later)
- **Git** (optional, needed only if you want to capture git repository state)

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
capsula-cli 0.10.0-alpha.1
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

## Installation Troubleshooting

### "cargo: command not found"

This means Rust is not installed or not in your PATH. Install Rust from [rustup.rs](https://rustup.rs/) and restart your terminal.

### Compilation Errors

If you encounter compilation errors, try:

1. **Update Rust:**
   ```bash
   rustup update
   ```

2. **Clean cargo cache:**
   ```bash
   cargo clean
   ```

3. **Try installing again:**
   ```bash
   cargo install capsula-cli --locked
   ```

### Permission Errors

If you get permission errors, cargo might be trying to install to a system directory. Make sure your Rust installation is set up for user-level installations (the default with rustup).

### Slow Installation

The first installation may take several minutes as cargo compiles Capsula and all its dependencies. Subsequent updates will be faster.

## Uninstalling Capsula

If you need to uninstall Capsula:

```bash
cargo uninstall capsula-cli
```

This removes the Capsula binary but doesn't delete any captured data in your `.capsula` directories.

## What's Next?

Now that Capsula is installed, let's run your first command!

[:octicons-arrow-right-24: Continue to Getting Started](getting-started.md)
