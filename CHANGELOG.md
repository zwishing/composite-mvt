# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-07-18

### Added

- Source metadata parsing from representative raw, gzip, Zstandard, and explicitly configured
  Brotli MVT samples.
- Optional gzip (default), Zstandard, and Brotli source decoding and complete-output encoding.
- Immutable `MvtComposer` construction with source ID, layer, compression-feature, and output
  validation.
- Independent duplicate-layer validation with configurable `Allow` and `Error` policies.
- Lock-free sharing of a built composer and independent request results.
- One-pass, whole-composite output compression suitable for HTTP `Content-Encoding` delivery.
- Rustdoc, README, example, feature-matrix, and package verification for the first publishable
  crate, with a documented Rust 1.87 MSRV. Exact Rust 1.87 verification remains pending because
  the configured mirror returned HTTP 404 and the official-source cargo download stalled.
