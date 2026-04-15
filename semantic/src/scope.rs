use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use parser::ast::{self, ParamList, Statement};

use crate::{
    hir::{FunctionRef, TopLevelHirScope, TypeRef, VarRef},
    top_level::TopLevelSymbolScope,
};

#[derive(Clone, Debug)]
pub enum Reference {
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

    pub fn get_reference(&self, name: &String) -> Option<Reference> {
        match self {
            Scope::TopLevel(tl) => tl.get_reference(name),
            Scope::Local(l) => l
                .variables
                .lock()
                .unwrap()
                .get(name)
                .cloned()
                .map(Reference::Var),
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
                Statement::VarDecl { name, init, .. } => {
                    if let Some(init) = init {
                        self.variables.lock().unwrap().insert(
                            name.clone(),
                            Arc::new(VarRef {
                                name: name.to_string(),
                                var_type: self.get_type_from_expr(init).unwrap(),
                            }),
                        );
                    }
                }
                _ => {}
            }
        }
    }

    pub fn get_type_from_expr(self: &Arc<Self>, expr: &ast::Expression) -> Option<Arc<TypeRef>> {
        match expr {
            ast::Expression::IntLiteral(_) => {
                self.parent.get_reference(&"int".to_string()).map(|r| {
                    if let Reference::Type(tr) = &r {
                        tr.clone()
                    } else {
                        unimplemented!()
                    }
                })
            }
            _ => unimplemented!(),
        }
    }

    pub fn with_params(self: &Arc<Self>, param_list: &ParamList) {
        for (param_name, type_expr) in &param_list.0 {
            match type_expr {
                parser::ast::TypeExpr::Identifier(id) => match self.parent.get_reference(id) {
                    Some(Reference::Type(type_ref)) => {
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
