use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use parser::ast::{self, ParamList, Postfix, Statement};

use crate::{
    hir::{TopLevelHirScope, TypeRef, VarRef},
    top_level::TopLevelSymbolScope,
};

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

    pub fn get_reference(&self, name: &String) -> Option<Arc<VarRef>> {
        match self {
            Scope::TopLevel(tl) => tl.get_reference(name),
            Scope::Local(l) => l.get_reference(name),
        }
    }

    pub fn next_var_idx(&self) -> usize {
        match self {
            Scope::TopLevel(tl) => tl.next_var_idx(),
            Scope::Local(l) => l.parent.next_var_idx(),
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

    pub fn get_reference(&self, name: &String) -> Option<Arc<VarRef>> {
        self.variables
            .lock()
            .unwrap()
            .get(name)
            .cloned()
            .or_else(|| self.parent.get_reference(name))
    }

    pub fn with_block(self: &Arc<Self>, block: &mut parser::ast::Block) {
        for statement in &mut block.0 {
            match statement {
                Statement::Block(b) => self.clone().with_block(b),
                Statement::VarDecl { name, init, .. } => {
                    if let Some(init) = init {
                        let var_type = self.get_type_from_expr(init).unwrap();

                        self.variables.lock().unwrap().insert(
                            name.clone(),
                            Arc::new(VarRef {
                                name: name.to_string(),
                                var_type,
                            }),
                        );
                    }
                }
                _ => {}
            }
        }
    }

    pub fn walk_postfixes(
        self: &Arc<Self>,
        initial: &mut Box<ast::Expression>,
        postfixes: &mut Vec<Postfix>,
    ) -> Option<Arc<TypeRef>> {
        let mut current_type = self.get_type_from_expr(initial)?;

        for postfix in postfixes {
            match postfix {
                Postfix::FieldAccess(id) => match &*current_type {
                    TypeRef::Struct(s) => {
                        let field = s.fields.get(id)?;
                        *id = field.name.clone();
                        current_type = field.var_type.clone();
                    }
                    _ => unimplemented!(),
                },
                Postfix::Call(_args) => match &*current_type {
                    TypeRef::Function(s) => {
                        // TODO: arg checking
                        current_type = s.return_type.clone()?;
                    }
                    _ => unimplemented!(),
                },
                _ => unimplemented!(),
            }
        }

        Some(current_type)
    }

    pub fn get_type_from_expr(
        self: &Arc<Self>,
        expr: &mut ast::Expression,
    ) -> Option<Arc<TypeRef>> {
        match expr {
            ast::Expression::IntLiteral(_) => self
                .parent
                .get_reference(&"int".to_string())
                .map(|r| r.var_type.clone()),

            ast::Expression::FloatLiteral(_) => self
                .parent
                .get_reference(&"float".to_string())
                .map(|r| r.var_type.clone()),

            ast::Expression::BoolLiteral(_) => self
                .parent
                .get_reference(&"bool".to_string())
                .map(|r| r.var_type.clone()),

            ast::Expression::StringLiteral(_) => self
                .parent
                .get_reference(&"string".to_string())
                .map(|r| r.var_type.clone()),

            ast::Expression::Identifier(id) => {
                let rf = self.get_reference(id)?;
                *id = rf.name.clone();
                Some(rf.var_type.clone())
            }

            ast::Expression::Postfix { initial, postfixes } => {
                self.walk_postfixes(initial, postfixes)
            }
        }
    }

    pub fn with_params(self: &Arc<Self>, param_list: &ParamList) {
        for (param_name, type_expr) in &param_list.0 {
            match type_expr {
                parser::ast::TypeExpr::Identifier(id) => {
                    if let Some(var_type) =
                        self.parent.get_reference(id).map(|r| r.var_type.clone())
                    {
                        self.variables.lock().unwrap().insert(
                            param_name.clone(),
                            Arc::new(VarRef {
                                var_type,
                                name: param_name.clone(),
                            }),
                        );
                    }
                }
            }
        }
    }
}
