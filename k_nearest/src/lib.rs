#![doc = include_str!("../readme.md")]

mod adapter;
mod best_set;
mod kd_tree;
mod metric;

pub use adapter::Adapter;
pub use kd_tree::KDTree;
pub use metric::Metric;

pub use metric::EuclideanDistanceSquared;
