//! Pure domain logic for the Flipclock desktop clock.
//!
//! Rendering, windowing and persistence I/O live elsewhere; this crate only
//! turns a timestamp and a [`Settings`] value into display data and known
//! window geometry.

pub mod geometry;
pub mod mode;
pub mod reading;
pub mod settings;
pub mod tick;
