// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

pub mod backends;
pub mod random_access;
pub mod record;
pub mod sequential;

pub use random_access::RandomAccessReader;
pub use record::QualityCompute;
pub use sequential::SequentialReader;
