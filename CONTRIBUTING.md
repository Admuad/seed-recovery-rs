# Contributing to Seed Phrase Recovery Tool

Thank you for your interest in contributing to the Seed Phrase Recovery Tool! This document provides guidelines and instructions for contributors.

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Submitting Changes](#submitting-changes)
- [Reporting Issues](#reporting-issues)
- [Feature Requests](#feature-requests)

## 🤝 Code of Conduct

This project adheres to the [Rust Code of Conduct](https://www.rust-lang.org/conduct.html). By participating, you are expected to uphold this code.

Please be respectful and inclusive in all interactions within this project.

## 🚀 Getting Started

### Prerequisites

- **Rust**: Version 1.70 or higher
- **Git**: For version control
- **GitHub Account**: For submitting pull requests

### Fork and Clone

1. **Fork the repository** on GitHub:
   ```bash
   # Visit https://github.com/Admuad/seed-recovery-rs and click "Fork"
   ```

2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/seed-recovery-rs.git
   cd seed-recovery-rs
   ```

3. **Add the original repository** as upstream:
   ```bash
   git remote add upstream https://github.com/Admuad/seed-recovery-rs.git
   ```

## 🔧 Development Setup

### Install Rust

If you haven't already installed Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### Install Development Dependencies

```bash
# Install Rust components for development
rustup component add clippy rustfmt

# Install additional tools (optional)
cargo install cargo-watch  # For auto-reloading during development
```

### Build and Test

```bash
# Build the project
cargo build --release

# Run tests
cargo test

# Run with specific features (if any)
cargo test --features "test-feature"

# Check code formatting
cargo fmt --check

# Lint the code
cargo clippy -- -D warnings
```

### Running During Development

```bash
# Run the tool (development mode)
cargo run --release

# Or use the built binary
./target/release/seed-recovery-rs
```

## 📝 Submitting Changes

### Workflow

1. **Create a new branch** for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/your-fix-name
   ```

2. **Make your changes** following the guidelines below.

3. **Test your changes**:
   ```bash
   # Build and test
   cargo build --release
   cargo test
   
   # Format code
   cargo fmt
   
   # Lint code
   cargo clippy -- -D warnings
   ```

4. **Commit your changes**:
   ```bash
   git add .
   git commit -m "feat: add your feature description"
   ```

   **Follow [Conventional Commits](https://www.conventionalcommits.org/)**:
   - `feat:` for new features
   - `fix:` for bug fixes
   - `docs:` for documentation changes
   - `style:` for code style changes
   - `refactor:` for code refactoring
   - `test:` for test changes
   - `chore:` for maintenance tasks

5. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

6. **Create a Pull Request**:
   - Go to the original repository: https://github.com/Admuad/seed-recovery-rs
   - Click "New Pull Request"
   - Select your branch and create the PR

### Code Style Guidelines

#### Rust Code Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Write documentation for all public APIs
- Include examples in documentation

#### Example Good Code:

```rust
/// Recovers a single missing word from a seed phrase using parallel processing.
///
/// # Arguments
/// * `wordlist` - The BIP-39 wordlist to use for recovery
/// * `seed_words` - The partial seed phrase with missing words marked as "?"
/// * `missing_index` - The index of the missing word
/// * `target` - Optional target address to match against
/// * `pb` - Progress bar for displaying recovery progress
/// * `chain` - The blockchain chain type
/// * `derivation_path` - The derivation path to use
///
/// # Returns
/// `Result<Option<(String, String, Vec<String>, Option<String>)>>` - 
/// The recovered seed phrase, address, missing words, and optional balance
///
/// # Example
/// ```rust
/// let result = recover_single_missing_parallel(
///     &wordlist,
///     &seed_words,
///     missing_index,
///     Some(&target_address),
///     &progress_bar,
///     &chain,
///     &derivation_path
/// )?;
/// ```
pub fn recover_single_missing_parallel(
    wordlist: &[String],
    seed_words: &[String],
    missing_index: usize,
    target: Option<&String>,
    pb: &ProgressBar,
    chain: &Chain,
    derivation_path: &DerivationPath,
) -> Result<Option<(String, String, Vec<String>, Option<String>)>> {
    // Implementation...
}
```

#### Documentation Requirements

- All public functions must have documentation
- Include examples for complex functions
- Document panic possibilities
- Explain parameters and return values

### Testing Guidelines

#### Unit Tests

- Write unit tests for all functions
- Test edge cases and error conditions
- Use descriptive test names
- Group related tests in modules

#### Example Test:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recover_single_missing_word() {
        let wordlist = get_bip39_wordlist();
        let seed_words = vec![
            "abandon".to_string(),
            "ability".to_string(),
            "able".to_string(),
            "about".to_string(),
            "above".to_string(),
            "absent".to_string(),
            "absorb".to_string(),
            "abstract".to_string(),
            "absurd".to_string(),
            "abuse".to_string(),
            "accident".to_string(),
            "?".to_string(), // Missing word
        ];
        
        let result = recover_single_missing_parallel(
            &wordlist,
            &seed_words,
            11, // missing index
            None,
            &mock_progress_bar(),
            &mock_chain(),
            &mock_derivation_path(),
        ).unwrap();
        
        assert!(result.is_some());
    }
    
    #[test]
    fn test_invalid_seed_phrase_length() {
        let result = validate_seed_phrase_length(&["short".to_string()], 12);
        assert!(result.is_err());
    }
}
```

#### Integration Tests

- Add integration tests in `tests/` directory
- Test the complete workflow
- Test with real blockchain data when possible

## 🐛 Reporting Issues

### Bug Reports

When reporting bugs, please include:

1. **Environment Information**:
   ```bash
   # Operating system
   uname -a
   
   # Rust version
   rustc --version
   
   # Cargo version
   cargo --version
   ```

2. **Steps to Reproduce**:
   ```bash
   # What you did
   ./seed-recovery-rs
   # 1. Selected EVM
   # 2. Chose BIP44
   # 3. Entered target address
   # 4. Pasted seed phrase: "word1 word2 ? ..."
   # 5. Got error: "..."
   ```

3. **Expected Behavior**:
   - What should have happened?

4. **Actual Behavior**:
   - What actually happened?
   - Error messages, stack traces, etc.

5. **Additional Context**:
   - Screenshots if applicable
   - Any other relevant information

### Issue Template

```markdown
## Bug Description
A brief description of the bug.

## Environment
- OS: [e.g., Ubuntu 22.04]
- Rust: [e.g., 1.70.0]
- Tool Version: [e.g., 0.3.0]

## Steps to Reproduce
1. First step
2. Second step
3. Error occurred

## Expected Behavior
What should have happened.

## Actual Behavior
What actually happened.

## Error Messages
```
Paste error messages here
```

## Additional Context
Any other information about the problem.
```

## 💡 Feature Requests

### Requesting Features

We welcome feature requests! Please:

1. **Check existing issues** first to avoid duplicates
2. **Use the feature request template**
3. **Explain the use case** clearly
4. **Consider implementation complexity**

### Feature Request Template

```markdown
## Feature Description
A clear and concise description of the feature.

## Use Case
Explain why this feature would be useful and who would benefit.

## Proposed Implementation
How do you envision this feature working?

## Alternatives
Are there any alternative solutions or workarounds?

## Additional Context
Any other context, screenshots, or examples.
```

### Feature Guidelines

- **Keep it focused**: One feature per request
- **Be realistic**: Consider the project scope and goals
- **Provide context**: Explain the problem you're solving
- **Be open to discussion**: Be willing to refine your idea

## 🏆 Recognition

Contributors will be recognized in:

- **README.md**: Major contributors section
- **CHANGELOG.md**: For significant changes
- **Git History**: All contributions are preserved
- **GitHub Contributors**: Automatic recognition by GitHub

## 📞 Getting Help

If you need help with contributing:

1. **Check existing issues** and discussions
2. **Create a new issue** with the "question" label
3. **Join our community discussions**
4. **Email the maintainers**: admuad@claw.dev

---

Thank you for contributing to the Seed Phrase Recovery Tool! 🎉

Every contribution helps make this tool better for everyone in the cryptocurrency community.