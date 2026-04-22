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
    pub fn from_top(
        path: &std::path::PathBuf,
        top_level: &Vec<parser::ast::TopLevel>,
    ) -> Arc<Self> {
        let tl = TopLevelSymbolScope::from(path, top_level);
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

    pub fn get_name(&self, export: bool) -> String {
        format!("{}{}", if export { 'V' } else { 'v' }, self.next_var_idx())
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
        let rf = self
            .variables
            .lock()
            .unwrap()
            .get(name)
            .cloned()
            .or_else(|| self.parent.get_reference(name));

        match rf {
            Some(v) => Some(v),
            None => {
                let var_ref = Arc::new(VarRef {
                    export: false,
                    name: name.clone(),
                    var_type: Arc::new(TypeRef::Name(name.clone())),
                });

                self.variables
                    .lock()
                    .unwrap()
                    .insert(name.clone(), var_ref.clone());

                Some(var_ref)
            }
        }
    }

    pub fn with_statement(self: &Arc<Self>, statement: &mut Statement) {
        match statement {
            Statement::Block(b) => self.clone().with_block(b),
            Statement::VarDecl { name, init, .. } => {
                if let Some(init) = init {
                    let var_type = self.get_type_from_expr(init).unwrap();

                    let var_name = name.clone();

                    *name = self.parent.get_name(false);

                    self.variables.lock().unwrap().insert(
                        var_name,
                        Arc::new(VarRef {
                            export: false,
                            name: name.clone(),
                            var_type,
                        }),
                    );
                }
            }
            Statement::Expression(e) => {
                self.get_type_from_expr(e);
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.get_type_from_expr(condition);
                self.clone().with_statement(then_branch);
                if let Some(else_branch) = else_branch {
                    self.clone().with_statement(else_branch);
                }
            }
            _ => {}
        }
    }

    pub fn with_block(self: &Arc<Self>, block: &mut parser::ast::Block) {
        for statement in &mut block.0 {
            self.with_statement(statement);
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
                    TypeRef::Package(p) => {
                        let var_ref = p.variables.get(id)?;
                        *id = var_ref.name.clone();
                        current_type = var_ref.var_type.clone();
                    }
                    _ => unimplemented!(),
                },
                Postfix::Call(args) => match &*current_type {
                    TypeRef::Function(s) => {
                        for arg in args {
                            self.get_type_from_expr(arg);
                        }
                        current_type = s.return_type.clone()?;
                    }
                    _ => unimplemented!(),
                },
                Postfix::Binary(op, right) => match op {
                    parser::ast::BinaryOp::Equal
                    | parser::ast::BinaryOp::NotEqual
                    | parser::ast::BinaryOp::GreaterThan
                    | parser::ast::BinaryOp::LessThan
                    | parser::ast::BinaryOp::GreaterThanOrEqual
                    | parser::ast::BinaryOp::LessThanOrEqual => {
                        self.get_type_from_expr(right)?;
                        current_type = Arc::new(TypeRef::Name("bool".to_string()));
                    }
                    parser::ast::BinaryOp::Plus
                    | parser::ast::BinaryOp::Minus
                    | parser::ast::BinaryOp::Multiply
                    | parser::ast::BinaryOp::Divide
                    | parser::ast::BinaryOp::Modulo => {
                        self.get_type_from_expr(right)?;
                    }
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
                .get_reference(&"int".to_string())
                .map(|r| r.var_type.clone()),

            ast::Expression::FloatLiteral(_) => self
                .get_reference(&"float".to_string())
                .map(|r| r.var_type.clone()),

            ast::Expression::BoolLiteral(_) => self
                .get_reference(&"bool".to_string())
                .map(|r| r.var_type.clone()),

            ast::Expression::StringLiteral(_) => self
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
        for (param_name, (export, type_expr)) in &param_list.0 {
            match type_expr {
                parser::ast::TypeExpr::Identifier(id) => {
                    if let Some(var_type) =
                        self.parent.get_reference(id).map(|r| r.var_type.clone())
                    {
                        self.variables.lock().unwrap().insert(
                            param_name.clone(),
                            Arc::new(VarRef {
                                export: *export,
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
