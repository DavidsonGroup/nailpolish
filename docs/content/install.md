---
title: Installation
---

# Installation

## Precompiled binary
Nailpolish is distributed as a single self-contained binary, with no dependencies beyond
libc. Binaries are published to the [Releases](https://github.com/DavidsonGroup/nailpolish/releases) page on GitHub.

=== "Linux"
    **Most recent stable release:**
    ```shell
    curl -LsSf "https://github.com/DavidsonGroup/nailpolish/releases/download/latest/nailpolish" -o nailpolish
    chmod +x nailpolish
    ```

    **Nightly release** (built from the `develop` branch)
    ```shell
    curl -LsSf "https://github.com/DavidsonGroup/nailpolish/releases/download/nightly/nailpolish" -o nailpolish
    chmod +x nailpolish
    ```

=== "macOS"
    **Most recent stable release:**
    ```shell
    curl -LsSf "https://github.com/DavidsonGroup/nailpolish/releases/download/nightly/nailpolish-macos-universal" -o nailpolish
    chmod +x nailpolish
    ```

    **Nightly release** (built from the `develop` branch)
    ```shell
    curl -LsSf "https://github.com/DavidsonGroup/nailpolish/releases/download/nightly/nailpolish-macos-universal" -o nailpolish
    chmod +x nailpolish
    ```

## Building from source

You will need a recent Rust toolchain, available from [rustup.rs](https://rustup.rs), and a modern C++ compiler.

```console
# must clone recursively or sync submodules
$ git clone --recursive https://github.com/DavidsonGroup/nailpolish.git && cd nailpolish

$ cargo build --release
```

The binary is written to `target/release/nailpolish`.

## Next steps

With Nailpolish installed, the [Quick Start](./quickstart.md) guide walks through
indexing a demo dataset and consensus calling its duplicates.
