# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.0](https://github.com/water-rs/native_executor/compare/v0.6.0...v0.7.0) - 2025-12-05

### Fixed

- *(ci)* update CARGO_REGISTRY_TOKEN in release workflow for consistency
- *(ci)* correct glob expansion in test script
- *(ci)* update android test to use x86_64 architecture
- resolve clippy warnings across codebase

### Other

- Run `cargo fmt`
- Update dependencies in Cargo.toml and enhance CI workflow by adding RUSTFLAGS to enforce warnings; refactor Android executor code for improved safety and readability
- Refactor CI workflow by removing the feature-checks job and updating Android emulator architecture to x86_64
- Update CI workflow by removing unnecessary features flag in build command for wasm target
- Remove changelog configuration from release settings to simplify release process
- Remove installation instructions from README.md and update release configuration to streamline version replacement process
- Enhance CI workflow by adding feature checks, updating build commands, and improving security audit steps; remove deprecated commit message format from release configuration
- Refactor CI workflow by removing feature checks and updating dependencies; clean up imports in test files for better readability
- Refactor linking for macOS and iOS to use dynamic libraries based on target OS
- Improve error handling in AndroidExecutor by dispatching to the UI thread
- Update release configuration to enable publishing and git tagging
- Add documentation job to CI workflow for generating Rust documentation
- Merge branch 'main' into patch-1
- Refactor CI workflows and update dependencies for improved testing and release processes
- Refactor Android main-thread registration by removing redundant thread check
- Update executor-core dependency to use 'dev' branch for Android and non-Android targets
- dev branch
- Enable CI to fail on Codecov errors
- Add CI and release workflows for automated testing and deployment
- Add Android, macOS, and WASM test scripts for cross-platform testing
- Add structured concurrency support, enhance polyfill tests, and update dependencies
- Refactor Cargo.toml dependencies, enhance Android executor initialization, and improve polyfill timer documentation
- Implement Android and Web platform executors, enhance polyfill functionality, and update Cargo.toml dependencies
- Refactor native-executor: Remove example files, consolidate platform executors, and enhance timer functionality
- Refactor default features in Cargo.toml and update README to reflect mailbox messaging
- Add polyfill feature documentation and improve polyfill executor implementation
