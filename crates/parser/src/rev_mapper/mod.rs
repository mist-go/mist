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

impl RustMap {
    pub fn shifted(self, lines: isize, cols: isize) -> Self {
        Self(
            self.0.saturating_add_signed(lines),
            self.1.saturating_add_signed(cols),
        )
    }
}

impl MistMap {
    pub fn shifted(self, lines: isize, cols: isize) -> Self {
        Self(
            self.0.saturating_add_signed(lines),
            self.1.saturating_add_signed(cols),
        )
    }
}

pub fn shift_rust_mappings(mappings: &mut HashSet<(RustMap, MistMap)>, lines: isize, cols: isize) {
    *mappings = mappings
        .iter()
        .map(|(rust, mist)| (rust.shifted(lines, cols), *mist))
        .collect();
}

pub fn shift_mist_mappings(mappings: &mut HashSet<(RustMap, MistMap)>, lines: isize, cols: isize) {
    *mappings = mappings
        .iter()
        .map(|(rust, mist)| (*rust, mist.shifted(lines, cols)))
        .collect();
}
