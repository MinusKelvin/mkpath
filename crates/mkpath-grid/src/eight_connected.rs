//! Types and utilities for working with 8-connected grid maps.

mod simple_expander;
mod jps_expander;

pub use self::simple_expander::*;
pub use self::jps_expander::*;

pub fn octile_distance(from: (i32, i32), to: (i32, i32)) -> f64 {
    let dx = (from.0 - to.0).abs();
    let dy = (from.1 - to.1).abs();
    let diagonals = dx.min(dy);
    let orthos = dx.max(dy) - diagonals;
    orthos as f64 + diagonals as f64 * std::f64::consts::SQRT_2
}
