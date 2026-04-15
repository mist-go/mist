use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use parser::ast::{ParamList, Statement};

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

#[derive(Debug)]
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
            Scope::Local(l) => l
                .variables
                .lock()
                .unwrap()
                .get(name)
                .cloned()
                .map(Refrence::Var),
        }
    }
}

#[derive(Debug)]
pub struct LocalScope {
    pub parent: Arc<Scope>,
    pub variables: Mutex<HashMap<String, Arc<VarRef>>>,
}

impl LocalScope {
    pub fn new(parent: Arc<Scope>) -> Arc<Self> {
        Arc::new(LocalScope {
            parent,
            variables: Mutex::new(HashMap::new()),
        })
    }

    pub fn with_block(self: &Arc<Self>, block: &mut parser::ast::Block) {
        for statement in &mut block.0 {
            match statement {
                Statement::Block(b) => self.clone().with_block(b),
                Statement::VarDecl { kind, name, init } => unimplemented!(),
                _ => {}
            }
        }
    }

    pub fn with_params(self: &Arc<Self>, param_list: &ParamList) {
        for (param_name, type_expr) in &param_list.0 {
            match type_expr {
                parser::ast::TypeExpr::Identifier(id) => match self.parent.get_refrence(id) {
                    Some(Refrence::Type(type_ref)) => {
                        self.variables.lock().unwrap().insert(
                            param_name.clone(),
                            Arc::new(VarRef {
                                var_type: type_ref,
                                name: param_name.clone(),
                            }),
                        );
                    }
                    _ => unimplemented!(),
                },
            }
        }
    }
}
