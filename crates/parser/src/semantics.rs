use crate::ast::{
    Block, ClassItem, Expression, FunctionDecl, Identifier, MatchItem, Postfix, Prefix, Spanned,
    Statement, StatementBranch, TopLevel, TopLevelKind, VarDeclStmt,
};

pub trait GetMutability {
    fn get_mutability(&self) -> Vec<Identifier>;
}

#[derive(Debug, Clone)]
pub struct SemanticError {
    pub line: usize,
    pub column: usize,
    pub error_message: String,
}

/// Class semantics - Ensuring fields are initialized

pub fn find_function_call<'a>(
    items: &'a Vec<ClassItem>,
    name: &Identifier,
) -> Option<&'a Spanned<FunctionDecl>> {
    items.iter().find_map(|v| {
        if let ClassItem::Method(m) = v {
            if m.item.self_param.is_some() && &m.item.name == name {
                Some(m)
            } else {
                None
            }
        } else {
            None
        }
    })
}

pub fn check_class_semantics<'a>(top_level: &TopLevel) -> Result<(), Vec<SemanticError>> {
    if let TopLevelKind::ClassDecl {
        fields,
        constructor,
        items,
        name,
        inherits,
        ..
    } = &top_level.0.item
    {
        if let Some(constructor) = constructor {
            let mut fields = fields
                .iter()
                .map(|field| Spanned {
                    item: field.item.decl.name.clone(),
                    line: field.line,
                    column: field.column,
                })
                .collect::<Vec<_>>();

            if inherits.is_some() {
                fields.push(Spanned {
                    line: top_level.0.line,
                    column: top_level.0.column,
                    item: Identifier("_super".to_string()),
                });
            }

            let mut mutability = constructor.item.body.get_mutability();

            let mut i = 0;
            while i < mutability.len() {
                let name = mutability[i].clone();
                if let Some(method) = find_function_call(items, &name) {
                    if let Some(body) = &method.item.body {
                        for entry in body.get_mutability() {
                            if !mutability.contains(&entry) {
                                mutability.push(entry);
                            }
                        }
                    }
                }
                i += 1;
            }

            let mut errs = Vec::new();

            for field in &fields {
                if !mutability.contains(&field.item) {
                    errs.push(SemanticError {
                        line: field.line,
                        column: field.column,
                        error_message: format!(
                            "class field `{}.{}` is uninitialized",
                            name.0, field.item.0
                        ),
                    });
                }
            }

            if errs.len() > 0 {
                return Err(errs);
            }
        }

        Ok(())
    } else {
        Ok(())
    }
}

impl<T: GetMutability> GetMutability for Vec<T> {
    fn get_mutability(&self) -> Vec<Identifier> {
        self.iter().flat_map(T::get_mutability).collect()
    }
}

impl GetMutability for Block {
    fn get_mutability(&self) -> Vec<Identifier> {
        let mut items: Vec<Identifier> = self
            .statements
            .iter()
            .flat_map(|v| v.item.get_mutability())
            .collect();

        if let Some(v) = &self.soft_return {
            items.append(&mut v.item.get_mutability());
        }

        items
    }
}

impl GetMutability for Statement {
    fn get_mutability(&self) -> Vec<Identifier> {
        match self {
            Statement::TopLevel(_) => Vec::new(),
            Statement::Block(body) | Statement::UnsafeBlock(body) => body.get_mutability(),
            Statement::If {
                initial,
                else_if,
                else_branch,
            } => {
                let mut all_sets: Vec<Vec<Identifier>> = Vec::new();
                all_sets.push(initial.body.get_mutability());

                for branch in else_if {
                    all_sets.push(branch.body.get_mutability());
                }

                if let Some(else_block) = else_branch {
                    all_sets.push(else_block.get_mutability());
                } else {
                    return Vec::new();
                }

                let mut result = all_sets.remove(0);
                for set in all_sets {
                    result.retain(|id| set.contains(id));
                }
                result
            }
            Statement::Loop(body) => body.get_mutability(),
            Statement::While(branch) => branch.body.get_mutability(),
            Statement::CStyleFor {
                body,
                init,
                condition,
                update,
                ..
            } => {
                let mut a = init.get_mutability();
                a.append(&mut condition.get_mutability());
                a.append(&mut update.get_mutability());
                a.append(&mut body.get_mutability());
                a
            }
            Statement::For { body, iterator, .. } => {
                let mut a = iterator.get_mutability();
                a.append(&mut body.get_mutability());
                a
            }
            Statement::Match(_, items) => {
                if items.is_empty() {
                    return Vec::new();
                }
                let mut all_sets: Vec<Vec<Identifier>> = items
                    .iter()
                    .map(|item| item.item.get_mutability())
                    .collect();
                let mut result = all_sets.remove(0);
                for set in all_sets {
                    result.retain(|id| set.contains(id));
                }
                result
            }
            Statement::VarDecl(stmt) => stmt
                .init
                .as_ref()
                .map(|e| e.get_mutability())
                .unwrap_or_default(),
            Statement::Return(expr) => expr
                .as_ref()
                .map(|e| e.get_mutability())
                .unwrap_or_default(),
            Statement::Break | Statement::Continue => Vec::new(),
        }
    }
}

impl GetMutability for StatementBranch {
    fn get_mutability(&self) -> Vec<Identifier> {
        let mut a = self.condition.get_mutability();
        a.append(&mut self.body.get_mutability());
        a
    }
}

impl GetMutability for MatchItem {
    fn get_mutability(&self) -> Vec<Identifier> {
        self.1.get_mutability()
    }
}

impl GetMutability for VarDeclStmt {
    fn get_mutability(&self) -> Vec<Identifier> {
        self.init
            .as_ref()
            .map(|e| e.get_mutability())
            .unwrap_or_default()
    }
}

fn get_self_ref(initial: &Expression, postfixes: &[Postfix]) -> Option<Identifier> {
    match initial {
        Expression::Path(v) => {
            let ident = v.0.first()?.ident.0.as_str();
            if ident == "self" || ident == "super" {
                if let Some(Postfix::FieldAccess(field, _)) = postfixes.first() {
                    if !matches!(postfixes.get(1), Some(Postfix::Call(_))) {
                        return Some(field.clone());
                    }
                }
            }
        }
        _ => {}
    }
    None
}

fn get_self_method_call(initial: &Expression, postfixes: &[Postfix]) -> Option<Identifier> {
    match initial {
        Expression::Path(v) => {
            let ident = v.0.first()?.ident.0.as_str();
            if ident == "self" || ident == "super" {
                if let Some(Postfix::FieldAccess(method, _)) = postfixes.first() {
                    if matches!(postfixes.get(1), Some(Postfix::Call(_))) {
                        return Some(method.clone());
                    }
                }
            }
        }
        _ => {}
    }
    None
}

impl Expression {
    fn get_self_ref(&self) -> Option<Identifier> {
        match self {
            Self::Fix {
                initial, postfixes, ..
            } => get_self_ref(initial, postfixes),
            _ => None,
        }
    }
}

impl GetMutability for Expression {
    fn get_mutability(&self) -> Vec<Identifier> {
        match self {
            Self::Array(v) => v.get_mutability(),
            Self::ArrayRepeat(v, i) => {
                let mut a = v.get_mutability();
                a.append(&mut i.get_mutability());
                a
            }
            Self::Binary { lhs, op, rhs } => {
                if mutates_lhs(op) {
                    let mut a = if let Some(self_ref) = lhs.get_self_ref() {
                        vec![self_ref]
                    } else {
                        lhs.get_mutability()
                    };

                    a.append(&mut rhs.get_mutability());
                    a
                } else {
                    let mut a = lhs.get_mutability();
                    a.append(&mut rhs.get_mutability());
                    a
                }
            }
            Self::Statement(stmt) => stmt.get_mutability(),
            Self::Fix {
                initial,
                prefixes,
                postfixes,
            } => {
                let mut result = Vec::new();

                // &mut self.field / &mut super.field / &mut super pattern
                if prefixes.iter().any(|p| matches!(p, Prefix::RefMut)) {
                    if let Some(self_ref) = get_self_ref(initial, postfixes) {
                        result.push(self_ref);
                    } else if let Expression::Path(v) = &**initial {
                        if v.0.first().map_or(false, |s| s.ident.0 == "super") {
                            result.push(Identifier("_super".to_string()));
                        }
                    }
                }

                // Recurse into all postfix sub-expressions
                for postfix in postfixes {
                    match postfix {
                        Postfix::Call(args) => {
                            for arg in args {
                                result.append(&mut arg.get_mutability());
                            }
                        }
                        Postfix::StructCall(fields) => {
                            for (_, expr) in fields {
                                if let Some(expr) = expr {
                                    result.append(&mut expr.get_mutability());
                                }
                            }
                        }
                        Postfix::Index(expr) => {
                            result.append(&mut expr.get_mutability());
                        }
                        _ => {}
                    }
                }

                // Method calls on self: self.method_name(args)
                if let Some(method) = get_self_method_call(initial, postfixes) {
                    result.push(method);
                }

                result
            }
            Self::Closure { body, .. } => body.get_mutability(),
            Self::Literal(_) => Vec::new(),
            Self::Path(v) => {
                if v.0.first().map_or(false, |s| s.ident.0 == "super") {
                    vec![Identifier("_super".to_string())]
                } else {
                    Vec::new()
                }
            }
        }
    }
}

pub fn mutates_lhs(op: &str) -> bool {
    matches!(op, "=" | "->")
}
