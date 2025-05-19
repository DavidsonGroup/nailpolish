use spoa::AlignmentResult;

/// Implements the logic deciding whether an alignment should be clustered or not
/// Should be used with `spoa::Graph::predict_alignment`
pub fn should_cluster(result: &AlignmentResult) -> bool {
    let insert_node_ratio = result.new_nodes as f32 / result.valid_nodes as f32;
    (result.valid_nodes > 25) && (insert_node_ratio < 0.25)
}
