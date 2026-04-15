use std::sync::Arc;

use crate::top_level::TopLevelScope;

pub enum Scope {
    TopLevel(TopLevelScope),
}

impl Scope {
    pub fn from_top(top_level: &Vec<parser::ast::TopLevel>) -> Arc<Self> {
        Arc::new(Self::TopLevel(TopLevelScope::from(top_level)))
    }
}
