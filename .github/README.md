# CI/CD Configuration

This directory contains GitHub Actions workflows for building, testing, and releasing Clip Vault.

## Workflows

### 🚀 Release Workflows

#### `release.yml`
- **Trigger**: Git tags starting with `v*` (e.g., `v1.0.0`)
- **Platforms**: 
  - macOS (Intel x86_64 and Apple Silicon aarch64)
  - Linux (x86_64)
  - Windows (x86_64)
- **Output**: Draft GitHub release with platform-specific binaries

#### `release-universal.yml`
- **Trigger**: Git tags starting with `v*`
- **Features**: Creates universal macOS binaries supporting both Intel and Apple Silicon
- **Output**: Optimized releases for better macOS distribution

### 🧪 Development Workflows

#### `build.yml`
- **Trigger**: Push to `main`/`develop` branches, Pull Requests
- **Purpose**: 
  - Type checking and linting
  - Cross-platform build verification
  - Rust tests and formatting checks
- **Platforms**: macOS, Linux, Windows

#### `nightly.yml`
- **Trigger**: Daily at 2 AM UTC, manual dispatch
- **Purpose**: Automated nightly builds from latest main branch
- **Output**: Pre-release builds with nightly versioning

### 📦 Dependency Management

#### `dependabot.yml`
- **Purpose**: Automated dependency updates
- **Scope**: 
  - Rust dependencies (Cargo.toml)
  - Node.js dependencies (package.json)
  - GitHub Actions versions
- **Schedule**: Weekly updates with grouped PRs

## Build Targets

| Platform | Architecture | Target | Binary Format |
|----------|-------------|--------|---------------|
| macOS    | Intel       | x86_64-apple-darwin | .app, .dmg |
| macOS    | Apple Silicon | aarch64-apple-darwin | .app, .dmg |
| macOS    | Universal   | universal-apple-darwin | .app, .dmg |
| Linux    | x86_64      | x86_64-unknown-linux-gnu | .deb, .AppImage |
| Windows  | x86_64      | x86_64-pc-windows-msvc | .exe, .msi |

## Release Process

1. **Create a tag**: `git tag v1.0.0 && git push origin v1.0.0`
2. **Automatic build**: GitHub Actions builds for all platforms
3. **Draft release**: Review generated release notes and binaries
4. **Publish**: Make the release public when ready

## Development Setup

The workflows automatically handle:
- Node.js and Rust toolchain setup
- Dependency caching for faster builds
- Platform-specific system dependencies
- Cross-compilation configuration

## Security Notes

- All workflows use pinned action versions for security
- Builds run in isolated environments
- Only necessary permissions are granted
- Release artifacts are automatically signed where supported

## Troubleshooting

Common issues and solutions:

### Build Failures
- Check system dependencies are properly installed
- Verify Rust and Node.js versions match requirements
- Review dependency compatibility

### Release Issues
- Ensure tag follows semantic versioning (v1.0.0)
- Check GitHub token permissions
- Verify no existing release with same tag

### Platform-Specific Problems
- **macOS**: Code signing may require developer certificates
- **Linux**: SQLCipher and WebKit dependencies must be available
- **Windows**: MSVC toolchain required for compilation