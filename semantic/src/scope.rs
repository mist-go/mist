use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use parser::ast::Statement;

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
                _ => {}
            }
        }
    }
}

pub fn walk_ast(top_scope: Arc<Scope>, tl: &mut Vec<parser::ast::TopLevel>) {
    for tl in tl {
        match tl {
            parser::ast::TopLevel::Import(_) => unimplemented!(),

            parser::ast::TopLevel::FunctionDecl {
                export,
                name,
                params,
                return_type,
                body,
            } => {
                let scope = LocalScope::new(top_scope.clone());
                scope.with_block(body);

                println!("{:?}", scope);
            }

            parser::ast::TopLevel::StructDecl {
                export,
                name,
                fields,
            } => {
                
            }
        }
    }
}
