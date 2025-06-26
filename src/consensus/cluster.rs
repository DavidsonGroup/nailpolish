// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

use spoa::AlignmentResult;

/// Implements the logic deciding whether an alignment should be clustered or not
/// Should be used with `spoa::Graph::predict_alignment`
pub fn should_cluster(result: &AlignmentResult) -> bool {
    let insert_node_ratio = result.new_nodes as f32 / result.valid_nodes as f32;
    (result.valid_nodes > 25) && (insert_node_ratio < 0.25)
}
