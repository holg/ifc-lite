<p align="center">
  <img src="docs/assets/logo.svg" alt="IFC-Lite Logo" width="120" height="120">
</p>

<h1 align="center">IFC-Lite (Bevy Fork)</h1>

<p align="center">
  <strong>Pure Rust/WebAssembly IFC viewer with Bevy 3D rendering</strong>
</p>

<p align="center">
  <a href="https://github.com/dbsystel/ifc-lite/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MPL--2.0-blue?style=flat-square" alt="License"></a>
  <a href="https://crates.io/crates/ifc-lite-core"><img src="https://img.shields.io/crates/v/ifc-lite-core?style=flat-square&logo=rust&label=core" alt="crates.io"></a>
</p>

---

> **Fork Notice:** This is a fork of [louistrue/ifc-lite](https://github.com/louistrue/ifc-lite), reimplemented as a **pure Rust/WASM** application. The original project uses Node.js/TypeScript with pnpm. This fork eliminates all JavaScript tooling in favor of:
> - **Yew** for the web UI
> - **Bevy** for 3D rendering (WebGPU/WebGL2)
> - **trunk** for WASM builds
>
> No Node.js, no pnpm, no npm packages required.

---

## Overview

**IFC-Lite** is a high-performance IFC (Industry Foundation Classes) viewer built entirely in **Rust**, compiled to **WebAssembly** for browser deployment. It uses **Bevy** for GPU-accelerated 3D rendering and **Yew** for the reactive web UI.

## Features

| Feature | Description |
|---------|-------------|
| **Pure Rust Stack** | No JavaScript build tools - just Rust, cargo, and trunk |
| **Bevy 3D Renderer** | WebGPU/WebGL2 rendering with orbit/pan/zoom camera controls |
| **Yew UI** | Reactive web interface with hierarchy panel, properties panel, and toolbar |
| **STEP/IFC Parsing** | Zero-copy tokenization with full IFC4 schema support |
| **URL Loading** | Load IFC files via `?file=model.ifc` URL parameter |
| **Streaming Pipeline** | Progressive geometry processing for large models |

## Quick Start

### Prerequisites

- **Rust** toolchain (rustup)
- **trunk** (`cargo install trunk`)
- **wasm-bindgen-cli** (`cargo install wasm-bindgen-cli`)
- Optional: **wasm-opt** for optimization, **brotli** for compression

### Build & Run

```bash
# Clone the repository
git clone https://github.com/dbsystel/ifc-lite.git
cd ifc-lite

# Build and serve locally
./scripts/build-wasm-split.sh serve

# Or build for production deployment
./scripts/build-wasm-split.sh deploy
```

### URL Parameters

Load IFC files directly via URL:
```
https://your-server.com/?file=model.ifc        # Loads /ifc/model.ifc
https://your-server.com/?file=path/to/file.ifc # Loads /ifc/path/to/file.ifc
```

## Project Structure

```
ifc-lite/
├── rust/                      # Core Rust libraries
│   ├── core/                  # IFC/STEP parsing
│   └── geometry/              # Geometry processing
│
├── crates/                    # WASM application crates
│   ├── ifc-lite-viewer/       # Main Yew application entry point
│   ├── ifc-lite-yew/          # Yew UI components & state
│   └── ifc-lite-bevy/         # Bevy 3D renderer
│
├── scripts/
│   ├── build-wasm-split.sh    # Build script for split WASM bundles
│   └── build-config.toml      # Build configuration
│
└── tests/ifc/                 # Test IFC files
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Browser                               │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐    localStorage    ┌─────────────────┐ │
│  │   Yew UI        │◄──────────────────►│  Bevy Renderer  │ │
│  │  (ifc-lite-yew) │     (bridge)       │ (ifc-lite-bevy) │ │
│  └────────┬────────┘                    └────────┬────────┘ │
│           │                                      │          │
│           ▼                                      ▼          │
│  ┌─────────────────┐                    ┌─────────────────┐ │
│  │  IFC Parser     │                    │  WebGPU/WebGL2  │ │
│  │ (ifc-lite-core) │                    │   Rendering     │ │
│  └─────────────────┘                    └─────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

- **Yew UI** handles file loading, hierarchy tree, property display, and user interactions
- **Bevy Renderer** provides 3D visualization with camera controls and entity picking
- **localStorage bridge** synchronizes geometry, selection, and visibility between the two WASM modules

## Rust Crates

| Crate | Description |
|-------|-------------|
| `ifc-lite-core` | STEP/IFC parsing with full IFC4 schema |
| `ifc-lite-geometry` | Mesh triangulation and extrusion |
| `ifc-lite-yew` | Yew UI components and state management |
| `ifc-lite-bevy` | Bevy 3D renderer plugin |
| `ifc-lite-viewer` | Main WASM entry point |

## Build Configuration

The build is configured via `scripts/build-config.toml`:

```toml
[project]
name = "ifc-lite"

[paths]
wasm_crate = "crates/ifc-lite-viewer"
bevy_crate = "crates/ifc-lite-bevy"
watch_paths = ["rust/core", "rust/geometry", "crates/ifc-lite-yew"]

[bundles]
leptos = true   # Yew UI (loads immediately)
bevy = true     # Bevy 3D (loads on demand)
```

## Development

```bash
# Check all crates compile
cargo check --workspace

# Run tests
cargo test --workspace

# Build Yew viewer only (faster iteration)
cd crates/ifc-lite-viewer && trunk serve

# Full production build
./scripts/build-wasm-split.sh

# Deploy to server
./scripts/build-wasm-split.sh deploy
```

## Browser Requirements

| Browser | Minimum Version | WebGPU | WebGL2 |
|---------|----------------|--------|--------|
| Chrome | 113+ | Yes | Yes |
| Edge | 113+ | Yes | Yes |
| Firefox | 127+ | Yes | Yes |
| Safari | 18+ | Yes | Yes |

## License

This project is licensed under the [Mozilla Public License 2.0](LICENSE).

## Acknowledgments

- Original project: [louistrue/ifc-lite](https://github.com/louistrue/ifc-lite)
- [Bevy](https://bevyengine.org/) game engine for 3D rendering
- [Yew](https://yew.rs/) framework for reactive web UI
- [nom](https://github.com/rust-bakery/nom) for parsing
- [earcutr](https://github.com/nickel-org/earcutr) for polygon triangulation
- [nalgebra](https://nalgebra.org/) for linear algebra

---

<p align="center">
  Made with Rust for the AEC industry
</p>
