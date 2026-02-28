//! Web visualizer module.
//!
//! Provides a browser-based graph visualization for duumbi semantic graphs.
//! Includes an axum HTTP server, WebSocket live sync, file watching, and
//! Cytoscape.js-based frontend rendering with dagre layout.

pub mod serialize;
pub mod server;
pub mod watcher;
pub mod ws;
