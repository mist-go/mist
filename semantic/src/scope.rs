use std::{collections::HashMap, sync::Arc};

use crate::{
    hir::{FunctionRef, TopLevelHirScope, TypeRef, VarRef},
    top_level::TopLevelSymbolScope,
};

#[derive(Clone, Debug)]
pub enum Refrence {
    Type(Arc<TypeRef>),
    Var(Arc<VarRef>),
    Func(Arc<FunctionRef>),
}

#[derive(Clone, Debug)]
pub enum Scope {
    TopLevel(TopLevelHirScope),
    Local(LocalScope),
}

impl Scope {
    pub fn from_top(top_level: &Vec<parser::ast::TopLevel>) -> Arc<Self> {
        let tl = TopLevelSymbolScope::from(top_level);
        Arc::new(Self::TopLevel(TopLevelHirScope::from_tlss(&tl)))
    }

    pub fn get_refrence(&self, name: &String) -> Option<Refrence> {
        match self {
            Scope::TopLevel(tl) => tl.get_refrence(name),
            Scope::Local(l) => l.variables.get(name).cloned().map(Refrence::Var),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LocalScope {
    pub parent: Arc<Scope>,
    pub variables: HashMap<String, Arc<VarRef>>,
}

impl LocalScope {
    pub fn from_block(parent: Arc<Scope>) -> Arc<Self> {
        Arc::new(LocalScope {
            parent,
            variables: HashMap::new(),
        })
    }
}
