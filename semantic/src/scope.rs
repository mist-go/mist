use std::sync::Arc;

use crate::{hir::TopLevelHirScope, top_level::TopLevelSymbolScope};

#[derive(Clone, Debug)]
pub enum Scope {
    TopLevel(TopLevelHirScope),
}

impl Scope {
    pub fn from_top(top_level: &Vec<parser::ast::TopLevel>) -> Arc<Self> {
        let tl = TopLevelSymbolScope::from(top_level);
        Arc::new(Self::TopLevel(TopLevelHirScope::from_tlss(&tl)))
    }
}
