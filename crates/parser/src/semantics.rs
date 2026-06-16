use crate::ast::{
    Block, ClassItem, Expression, FunctionDecl, Identifier, Postfix, Spanned, TopLevel,
    TopLevelKind,
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
            if m.item.is_using_self() && &m.item.name == name {
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
        ..
    } = &top_level.0.item
    {
        if let Some(constructor) = constructor {
            let fields = fields
                .iter()
                .map(|field| Spanned {
                    item: field.item.decl.name.clone(),
                    line: field.line,
                    column: field.column,
                })
                .collect::<Vec<_>>();

            let mut mutability = constructor.item.body.get_mutability();

            for name in mutability.clone() {
                if let Some(method) = find_function_call(items, &name) {
                    if let Some(body) = &method.item.body {
                        mutability.append(&mut body.get_mutability());
                    }
                }
            }

            let mut mutability_iter = mutability.into_iter();

            let mut errs = Vec::new();

            for field in &fields {
                if mutability_iter.find(|v| v == &field.item).is_none() {
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

impl Expression {
    fn get_self_ref(&self) -> Option<Identifier> {
        match self {
            Self::Fix {
                initial, postfixes, ..
            } => match &**initial {
                Self::Path(v) => {
                    if v.0[0].ident.0 == "self" {
                        if let Some(field) = postfixes.get(0) {
                            match field {
                                Postfix::FieldAccess(field, _) => return Some(field.clone()),
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }

        None
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
            _ => Vec::new(),
        }
    }
}

pub fn mutates_lhs(op: &str) -> bool {
    matches!(
        op,
        "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>=" | "->"
    )
}
