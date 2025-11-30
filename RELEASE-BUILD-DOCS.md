# Release Build Documentation

## Overview

This repository includes a comprehensive GitHub Workflow that automatically builds and releases save-steward for multiple platforms on every push to the main branch.

## Build Targets

The workflow builds for the following platforms:

- **Windows x64**: MSI installer
- **macOS Intel (x64)**: DMG installer  
- **macOS Apple Silicon (ARM64)**: DMG installer
- **Linux x64**: AppImage executable
- **Linux ARM64**: AppImage executable

## How It Works

### 1. Triggers
The workflow runs automatically when:
- Code is pushed to `main` or `master` branch
- Pull requests are created against `main` or `master`

### 2. Build Process
1. **Changelog Generation**: Automatically generates changelog from git commits
2. **Multi-platform Build**: Uses matrix strategy to build for all targets
3. **Dependency Caching**: Caches Rust and npm dependencies for faster builds
4. **Release Creation**: Creates GitHub releases with all platform binaries

### 3. Release Artifacts
Each release includes:
- Windows: `save-steward_VERSION_x64_en-US.msi`
- macOS Intel: `save-steward_VERSION_x64.dmg`
- macOS ARM64: `save-steward_VERSION_aarch64.dmg`
- Linux x64: `save-steward_VERSION_x86_64.AppImage`
- Linux ARM64: `save-steward_VERSION_aarch64.AppImage`

## Release Naming Convention

Releases are automatically named with the version from `src-tauri/Cargo.toml`:
- Example: `Save Steward v0.1.0`

## Release Notes

Each release includes:
- **Changes section**: Auto-generated from git commits since last tag
- **Download section**: Clear instructions for each platform
- **Artifact attachments**: All platform-specific installers

## System Dependencies

### Linux Build Dependencies
The workflow automatically installs:
- libgtk-3-dev
- libwebkit2gtk-4.0-dev
- libayatana-appindicator3-dev
- librsvg2-dev
- libssl-dev
- pkg-config
- build-essential
- curl, wget, file, unzip

### Build Requirements
- Node.js 18+
- Rust stable toolchain
- Tauri CLI

## Configuration

### Tauri Configuration
The build targets are configured in `src-tauri/tauri.conf.json`:
```json
{
  "bundle": {
    "active": true,
    "targets": "all"
  }
}
```

### Workflow Configuration
Key workflow settings:
- **Node.js version**: 18
- **Rust version**: stable
- **Build caching**: Enabled for faster subsequent builds
- **Failure strategy**: `fail-fast: false` (continue building other platforms if one fails)

## Manual Build

To build locally for testing:
```bash
# Install dependencies
npm install

# Build for current platform
npm run tauri build

# Build for specific target
cargo build --target x86_64-pc-windows-msvc
```

## Troubleshooting

### Common Issues
1. **Build failures**: Check that all system dependencies are installed
2. **Missing artifacts**: Verify Tauri configuration and bundle settings
3. **Caching issues**: Clear GitHub Actions cache or manually trigger rebuild

### Debug Information
The workflow includes detailed logging and error reporting to help diagnose build issues.

## Security

- Uses official GitHub Actions for dependency installation
- Employs dependency caching with secure hash-based keys
- No sensitive information is logged or exposed in build artifacts
