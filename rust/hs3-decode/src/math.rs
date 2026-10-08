#[inline]
pub(crate) fn round_f32(val: f32) -> f32 {
    if val >= 0.0 {
        ((val + 0.5) as i32) as f32
    } else {
        ((val - 0.5) as i32) as f32
    }
}

#[inline]
pub(crate) fn round_f64(val: f64) -> f64 {
    if val >= 0.0 {
        ((val + 0.5) as i64) as f64
    } else {
        ((val - 0.5) as i64) as f64
    }
}
