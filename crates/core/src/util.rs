//! util.rs - core 内部公共工具函数。

pub(crate) fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}
