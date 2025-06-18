# Release Instructions for xr2280x-hid

This document provides step-by-step instructions for releasing a new version of the xr2280x-hid crate to both GitHub and crates.io.

## Prerequisites

Before releasing, ensure you have:

1. **Crates.io Account**: Register at [crates.io](https://crates.io/) if you haven't already
2. **Crates.io API Token**: Generate an API token from your crates.io account settings
3. **Git Repository Access**: Push access to the GitHub repository
4. **Cargo Login**: Authenticate with crates.io using `cargo login <your-token>`

## Pre-Release Checklist

- [ ] All tests pass (`cargo test`)
- [ ] All examples build (`cargo build --examples`)
- [ ] Documentation builds without warnings (`cargo doc`)
- [ ] CHANGELOG.md is updated with new version
- [ ] Cargo.toml version is bumped
- [ ] All changes are committed to git
- [ ] Code review completed (if working with team)

## Release Process

### 1. Final Testing

```bash
# Run all tests
cargo test

# Build examples
cargo build --examples

# Check documentation
cargo doc --no-deps

# Lint the code
cargo clippy --all-targets --all-features -- -D warnings

# Check formatting
cargo fmt --check
```

### 2. Prepare the Git Repository

```bash
# Ensure you're on the main branch
git checkout main

# Pull latest changes
git pull origin main

# Create a commit for the version bump (if not already done)
git add Cargo.toml CHANGELOG.md
git commit -m "Bump version to 0.9.3"

# Push changes
git push origin main
```

### 3. Create a Git Tag

```bash
# Create an annotated tag for the release
git tag -a v0.9.3 -m "Release v0.9.3 - Multi-Device Selection Support"

# Push the tag
git push origin v0.9.3
```

### 4. Create GitHub Release

1. Go to your repository on GitHub: `https://github.com/tiborgats/xr2280x-hid`
2. Click on "Releases" in the right sidebar
3. Click "Create a new release"
4. Select the tag you just created: `v0.9.3`
5. Set the release title: `v0.9.3 - Multi-Device Selection Support`
6. Copy the relevant section from CHANGELOG.md into the release description:

```markdown
### Added
- **Multi-Device Selection Support**: Comprehensive device selection when multiple XR2280x devices are connected
  - `Xr2280x::enumerate_hardware_devices()` - Get list of all available XR2280x hardware devices
  - `Xr2280x::open_by_serial()` - Open device by serial number
  - `Xr2280x::open_by_index()` - Open device by enumeration index (0-based)
  - `Xr2280x::open_by_path()` - Open device by platform-specific path
- **Enhanced Error Handling**: Specific error types for multi-device selection failures
  - `Error::DeviceNotFoundBySerial` - Serial number not found
  - `Error::DeviceNotFoundByIndex` - Index out of range
  - `Error::DeviceNotFoundByPath` - Invalid device path
  - `Error::MultipleDevicesFound` - Ambiguous selection when expecting one device
- **Re-exported Types**: Essential hidapi types now available through the crate
  - `hidapi::DeviceInfo` and `hidapi::HidApi` re-exported for convenience
- **New Example**: `multi_device_selection.rs` demonstrating all selection methods

### Changed
- Refactored device opening logic to use unified `from_hid_devices()` method internally
- Updated documentation with comprehensive multi-device selection examples
- Enhanced README with dedicated multi-device selection section

### Fixed
- Improved consistency between different device opening methods

## New Features

🎉 **Multi-Device Selection**: You can now reliably work with multiple XR2280x devices connected to the same system. This release adds comprehensive device selection methods for production environments.

## Migration Guide

This release was **fully backward compatible** with existing code. However, in v0.10.0, the legacy logical device functions were completely removed in favor of the hardware device API. To use the current API:

```rust
// New: Enumerate all hardware devices
let devices = Xr2280x::enumerate_hardware_devices(&hid_api)?;

// New: Open by serial number
let device = Xr2280x::open_by_serial(&hid_api, "ABC123456")?;

// New: Open by index
let device = Xr2280x::open_by_index(&hid_api, 1)?;

// Updated: Use hardware device API
let device = Xr2280x::open_first_hardware(&hid_api)?;
```
```

7. Mark as pre-release if this is a beta/RC, otherwise leave unchecked
8. Click "Publish release"

### 5. Publish to Crates.io

```bash
# Dry run first to check for issues
cargo publish --dry-run --allow-dirty

# If dry run succeeds, publish for real
cargo publish --allow-dirty
```

### 6. Verify the Release

1. **Check crates.io**: Go to https://crates.io/crates/xr2280x-hid and verify the new version appears
2. **Test installation**: In a separate directory, test that the crate can be used:
   ```bash
   cargo new test-xr2280x
   cd test-xr2280x
   cargo add xr2280x-hid@0.9.3
   cargo check
   ```
3. **Check documentation**: Verify docs are building at https://docs.rs/xr2280x-hid/

## Post-Release Tasks

- [ ] Update any dependent projects to use the new version
- [ ] Announce the release on relevant forums/communities if this is a major release
- [ ] Update project README.md if installation instructions changed
- [ ] Consider writing a blog post for significant releases

## Troubleshooting

### Common Issues

**"crate already exists" error**: The version number hasn't been incremented, or you're trying to republish the same version.

**"authentication failed" error**: Your crates.io token has expired or is incorrect. Run `cargo login` again.

**"dirty working directory" error**: You have uncommitted changes. Commit or stash them before publishing.

**Documentation build fails**: Fix any documentation warnings and try again. Use `cargo doc` locally to test.

### Emergency Procedures

If you discover a critical issue after publishing:

1. **Do NOT delete the published version** (crates.io doesn't allow this)
2. **Immediately publish a patch version** (e.g., 0.9.2) with the fix
3. **Yank the problematic version** if it's dangerous:
   ```bash
   cargo yank --vers 0.9.3
   ```
4. **Document the issue** in CHANGELOG.md and GitHub releases

## Version History

- `v0.9.3` - Multi-Device Selection Support
- `v0.9.2` - Robust I2C Timeout System + Fast Stuck Bus Detection
- `v0.9.1` - Critical I2C address bug fix + modular refactor
- `v0.9.0` - Initial modular release

## Contact

For questions about the release process, contact the maintainer or create an issue on GitHub.
