# Changelog

All notable changes to the Seed Phrase Recovery Tool will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Placeholder for unreleased features

### Changed
- Placeholder for unreleased changes

### Fixed
- Placeholder for unreleased bug fixes

## [0.3.0] - 2026-03-24

### Added
- **Multiple Derivation Paths**: Support for BIP44, BIP49, BIP84, and custom paths
- **Smart Path Selection**: Chain-specific path filtering and recommendations
- **Custom Path Validation**: Input validation for user-defined derivation paths
- **Enhanced UI**: Interactive path selection interface
- **Network Abstraction**: Unified derivation interface across all chains
- **Professional Documentation**: Comprehensive README with usage examples
- **Contributing Guidelines**: Detailed contribution instructions
- **Version Upgrade**: Updated to v0.3.0

### Changed
- **Complete Rewrite**: Updated `src/chains.rs` with path support
- **UI Enhancement**: Updated `src/main.rs` with path selection
- **Recovery Functions**: All functions now accept `DerivationPath` parameter
- **Error Handling**: Improved error messages and validation
- **User Experience**: More intuitive workflow with path selection

### Fixed
- **Memory Management**: Fixed mutex lifetime issues in parallel processing
- **Integer Overflow**: Fixed potential overflow with 3 missing words (8.5B combinations)
- **API Compatibility**: Updated ed25519-dalek and k256 APIs

### Technical Details
- **New Enum**: `DerivationPath` with Standard, SegWitP2SH, SegWitNative, Custom variants
- **Chain Updates**: All chain structs now include `coin_type` field
- **Performance**: Maintained same parallel processing speed with new features
- **Backward Compatibility**: All existing functionality preserved

## [0.2.0] - 2026-03-21

### Added
- **RPC Balance Checking**: Real-time balance verification for all supported chains
- **7 Chain Support**: EVM, Solana, Sui, Aptos, Pi Network, Tron, Dogecoin
- **Parallel Processing**: Multi-threaded scanning using Rayon (10x+ speedup)
- **3 Missing Words**: Extended support from 2 to 3 missing words
- **Position Awareness**: Optimized search when missing word positions are known
- **Multi-Length Support**: 12, 15, 18, 21, 24-word seed phrases
- **Professional UI**: Beautiful terminal interface with ASCII art and progress bars
- **One-Command Setup**: Automated installation script with progress indicators

### Changed
- **Architecture**: Complete rewrite with modular design
- **Performance**: Dramatic speed improvements for all recovery scenarios
- **User Experience**: Intuitive CLI with clear prompts and feedback
- **Code Quality**: Type-safe Rust implementation with comprehensive error handling

### Technical Details
- **Dependencies**: Added Rayon for parallel processing, reqwest for RPC calls
- **Binary Size**: Optimized 1.1MB release binary
- **Memory Usage**: Efficient memory management for large combination spaces
- **Compilation**: Successfully builds with latest stable Rust

### Performance Benchmarks
- **1 missing word**: < 1 second (2,048 combinations)
- **2 missing words**: ~10-14 minutes (4,194,304 combinations)  
- **3 missing words**: ~9 hours known position, ~62 days unknown (8,589,934,592 combinations)

## [0.1.0] - 2026-03-15

### Added
- **Initial Release**: Basic seed phrase recovery functionality
- **EVM Support**: Ethereum and EVM-compatible chain recovery
- **1-2 Missing Words**: Support for recovering 1-2 missing words
- **Target Address Verification**: Match recovered addresses against known targets
- **BIP-39 Compliance**: Full support for standard BIP-39 wordlists
- **Command Line Interface**: Basic CLI for recovery operations

### Technical Details
- **Language**: Implemented in Rust for safety and performance
- **Algorithm**: Brute-force with BIP-39 wordlist validation
- **Output**: Recovered seed phrases and corresponding addresses
- **Security**: Offline operation with no data collection

## [0.0.1] - 2026-03-10

### Added
- **Project Initialization**: Repository setup and basic structure
- **Concept**: Initial design and architecture planning
- **Documentation**: Basic README and project description

---

## Versioning Philosophy

This project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html):

- **MAJOR version**: Incompatible API changes or major feature additions
- **MINOR version**: New functionality in a backward-compatible manner
- **PATCH version**: Backward-compatible bug fixes

### Version Categories

#### **Major Releases (X.0.0)**
- Breaking changes to API or CLI
- Complete feature overhauls
- Significant architecture changes

#### **Minor Releases (0.X.0)**
- New blockchain support
- New recovery features
- Performance improvements
- UI/UX enhancements

#### **Patch Releases (0.0.X)**
- Bug fixes
- Security patches
- Documentation updates
- Minor improvements

### Release Schedule

- **Major Releases**: Planned for v1.0.0 (stable production release)
- **Minor Releases**: Every 2-3 months with new features
- **Patch Releases**: As needed for bug fixes and security updates

### Compatibility承诺 (Compatibility Promise)

We strive to maintain backward compatibility within minor versions:

- **CLI Interface**: Breaking changes only in major versions
- **Configuration Files**: Backward-compatible when possible
- **Recovery Results**: Same output format within major versions
- **Dependencies**: Updated carefully to avoid breaking changes

---

**Changelog maintained by [Admuad](https://github.com/Admuad)**

*Last updated: 2026-03-24*