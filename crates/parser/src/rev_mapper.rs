use std::{collections::HashSet, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct MistMap(pub usize, pub usize);

#[derive(
    Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct RustMap(pub usize, pub usize);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapping {
    pub mist_path: PathBuf,
    pub map: HashSet<(RustMap, MistMap)>,
}

impl Mapping {
    pub fn new(mist_path: PathBuf) -> Self {
        Self {
            mist_path,
            map: HashSet::new(),
        }
    }

    pub fn find(&self, target: &RustMap) -> Option<(RustMap, MistMap)> {
        self.map
            .iter()
            .copied()
            .filter(|(rust, _)| rust <= target)
            .max_by_key(|(rust, _)| *rust)
    }

    pub fn find_by_mist(&self, target: &MistMap) -> Option<(RustMap, MistMap)> {
        self.map
            .iter()
            .copied()
            .filter(|(_, mist)| mist <= target)
            .max_by_key(|(_, mist)| *mist)
    }

    pub fn shift_rust(&mut self, lines: isize, cols: isize) {
        self.map = self
            .map
            .iter()
            .map(|(rust, mist)| (rust.shifted(lines, cols), *mist))
            .collect();
    }

    pub fn shift_mist(&mut self, lines: isize, cols: isize) {
        self.map = self
            .map
            .iter()
            .map(|(rust, mist)| (*rust, mist.shifted(lines, cols)))
            .collect();
    }
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
