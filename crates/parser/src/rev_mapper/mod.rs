use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(
    Debug, Default, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct MistMap(pub usize, pub usize);

#[derive(
    Debug, Default, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct RustMap(pub usize, pub usize);

pub fn find_mapping(
    mapping: &HashSet<(RustMap, MistMap)>,
    target: &RustMap,
) -> Option<(RustMap, MistMap)> {
    mapping
        .iter()
        .copied()
        .filter(|(rust, _)| rust <= target)
        .max_by_key(|(rust, _)| *rust)
}
