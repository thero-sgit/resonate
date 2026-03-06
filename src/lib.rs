//! Resonate: Audio fingerprinting service
//!
//! This library provides a complete audio fingerprinting pipeline with support for both
//! HTTP-based and Kafka event-driven processing.
//!
//! # Fingerprinting Pipeline
//!
//! The core fingerprinting functionality is organized in three stages:
//!
//! - **Decoding**: Audio files are decoded using Symphonia and resampled to mono at 11,025 Hz
//! - **Extraction**: Frames are windowed and analyzed via FFT to compute spectral magnitudes
//! - **Hashing**: Local spectral peaks are identified and paired to generate compact 64-bit hashes
//!
//! # Modules
//!
//! - [`fingerprint`]: Core fingerprinting pipeline and related submodules
//! - [`server`]: HTTP API routes and handlers
//! - [`streaming`]: Kafka integration for event-driven processing

pub mod streaming;
pub mod server;
pub mod fingerprint;