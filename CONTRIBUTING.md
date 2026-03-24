# Contributing to YubiTUI

Thank you for your interest in contributing to YubiTUI! This document provides guidelines and instructions for contributing.

## Code of Conduct

Be respectful, inclusive, and constructive. We're all here to make YubiKey management better.

## Getting Started

### Prerequisites

- **Rust 1.75+**: Install from [rustup.rs](https://rustup.rs/)
- **PC/SC Lite**: Smart card daemon
  - macOS: `brew install pcsc-lite`
  - Ubuntu/Debian: `sudo apt-get install libpcsclite-dev pcscd`
  - Arch: `sudo pacman -S pcsclite`
- **GPG 2.1+**: For OpenPGP operations
  - macOS: `brew install gnupg`
  - Ubuntu: `sudo apt-get install gnupg`

### Building from Source

```bash
git clone https://github.com/yourusername/yubitui
cd yubitui
cargo build --release
```

### Running Tests

```bash
# Unit tests (no hardware required)
cargo test

# Integration tests (requires YubiKey)
cargo test --features device-tests -- --ignored
```

## Development Workflow

1. **Fork** the repository
2. **Create a feature branch**: `git checkout -b feature/my-awesome-feature`
3. **Make your changes**
4. **Test thoroughly**: Ensure all tests pass and add new tests for new features
5. **Commit with meaningful messages**: Follow the commit message guidelines below
6. **Push** to your fork
7. **Create a Pull Request**

## Code Style

### Rust Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Run `cargo fmt` before committing
- Run `cargo clippy` and address warnings
- Use meaningful variable names
- Add documentation comments for public APIs

### Project Structure

```
src/
├── app.rs              # Application state and event loop
├── main.rs             # CLI entry point
├── yubikey/            # YubiKey operations
│   ├── detection.rs    # Device detection
│   ├── openpgp.rs      # OpenPGP card operations
│   ├── piv.rs          # PIV operations
│   ├── pin.rs          # PIN management
│   └── ssh.rs          # SSH configuration
├── diagnostics/        # System diagnostics
│   ├── gpg_agent.rs    # GPG agent checks
│   ├── pcscd.rs        # PC/SC daemon checks
│   ├── scdaemon.rs     # Scdaemon checks
│   └── ssh_agent.rs    # SSH agent checks
├── ui/                 # TUI rendering
│   ├── dashboard.rs    # Main dashboard
│   ├── diagnostics.rs  # Diagnostics view
│   ├── keys.rs         # Key management view
│   ├── pin.rs          # PIN management view
│   └── ssh.rs          # SSH wizard view
└── utils/              # Utilities
    ├── config.rs       # Configuration helpers
    └── gpg_cli.rs      # GPG CLI wrapper
```

## Commit Message Guidelines

Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Maintenance tasks
- `ci`: CI/CD changes

### Examples

```
feat(pin): add PIN retry counter display

Displays the current PIN retry counter on the dashboard with
color-coded warnings when retries are low.

Closes #42
```

```
fix(detection): handle YubiKey 5Ci detection

YubiKey 5Ci was incorrectly identified as YubiKey 5C.
Added specific check for "5ci" in reader name.

Fixes #38
```

## Adding Features

### New UI Screen

1. Create a new module in `src/ui/`
2. Implement `pub fn render(frame: &mut Frame, area: Rect, ...)`
3. Add screen variant to `Screen` enum in `src/app.rs`
4. Handle keyboard input in `App::handle_key_event()`
5. Add navigation entry in dashboard

### New YubiKey Operation

1. Add function to appropriate module in `src/yubikey/`
2. Handle errors gracefully with user-friendly messages
3. Add state to `YubiKeyState` if needed
4. Update UI to display new functionality
5. Add tests

### New Diagnostic Check

1. Add module to `src/diagnostics/`
2. Implement check function returning status struct
3. Add to `Diagnostics::run()` in `src/diagnostics/mod.rs`
4. Update diagnostics UI to display results
5. Add recommendations for failures

## Testing Guidelines

### Unit Tests

- Test individual functions in isolation
- Mock external dependencies (YubiKey, GPG, etc.)
- Place tests in the same file as the code: `#[cfg(test)] mod tests { ... }`

### Integration Tests

- Test full workflows end-to-end
- Place in `tests/` directory
- Mark hardware-dependent tests with `#[ignore]` and `#[cfg(feature = "device-tests")]`

### Example

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pin_retries() {
        let status = "PIN retry counter : 3 0 3";
        let (user, _, admin) = parse_pin_retries(status).unwrap();
        assert_eq!(user, 3);
        assert_eq!(admin, 3);
    }

    #[test]
    #[ignore]
    #[cfg(feature = "device-tests")]
    fn test_detect_yubikey() {
        // Requires physical YubiKey
        let keys = detect_yubikeys().unwrap();
        assert!(!keys.is_empty());
    }
}
```

## Documentation

- Add rustdoc comments for all public items
- Include examples in documentation
- Update README.md for user-facing changes
- Add inline comments for complex logic

## Security Considerations

When contributing, keep in mind:

- **Never log PINs or keys**: Use `[REDACTED]` in logs
- **Validate all user input**: Especially before passing to GPG
- **Use secure defaults**: E.g., touch policy, PIN caching
- **Document security implications**: Of new features
- **Test error paths**: Ensure no sensitive data leaks in errors

## Performance

- Profile before optimizing
- Use `cargo flamegraph` for profiling
- Avoid unnecessary allocations in the render loop
- Use efficient data structures
- Test with slow hardware

## Pull Request Process

1. **Ensure all tests pass**: `cargo test`
2. **Run formatting**: `cargo fmt`
3. **Run linter**: `cargo clippy`
4. **Update documentation**: README, rustdoc, etc.
5. **Add changelog entry**: If user-facing change
6. **Request review**: Tag relevant maintainers
7. **Address feedback**: Make requested changes
8. **Squash commits**: If requested

## Release Process

(For maintainers)

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create signed tag: `git tag -s v0.1.0 -m "Release v0.1.0"`
4. Push tag: `git push origin v0.1.0`
5. GitHub Actions will build and publish

## Getting Help

- **Questions**: Open a [discussion](https://github.com/yourusername/yubitui/discussions)
- **Bugs**: Open an [issue](https://github.com/yourusername/yubitui/issues)
- **Chat**: Join our [Discord](https://discord.gg/example)

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (MIT OR Apache-2.0).
