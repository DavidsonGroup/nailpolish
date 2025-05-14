use log::info;
use std::env;

/// Calculates a running average when a new value is added to an existing average
pub(crate) fn running_avg(existing: f32, new: f32, new_count: usize) -> f32 {
    let new_count = new_count as f32;
    (existing * (new_count - 1.0) + new) / new_count
}

/// Computes the arithmetic mean of a sequence of f32 values
pub(crate) fn complete_avg<I>(iter: I) -> f32
where
    I: Iterator<Item = f32>,
{
    let (sum, count) = iter.fold((0.0, 0), |(sum, count), val| (sum + val, count + 1));
    if count == 0 {
        0.0
    } else {
        sum / count as f32
    }
}
