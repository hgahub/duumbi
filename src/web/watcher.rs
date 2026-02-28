//! File watcher for live graph reloading.
//!
//! Uses `notify-debouncer-mini` to watch `.jsonld` files and trigger graph
//! rebuilds with a 200ms debounce interval.
