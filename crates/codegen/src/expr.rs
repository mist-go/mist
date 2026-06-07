use mist_parser::ast::*;

use crate::Context;

use crate::{GenRust, GetRust, RustCodegen};

impl GetRust for ExprPath {
    fn get_rust(&self) -> String {
        self.0
            .iter()
            .map(ExprPathSegment::get_rust)
            .collect::<Vec<_>>()
            .join("::")
    }

    fn get_rust_ctx(&self, cx: &mut Context) -> String {
        let mut a = self.clone();

        if let Some(path) = &cx.expr_super {
            if self.0[0].ident.0 == "super" {
                a.0[0].ident.0 = String::from("self._super");
            } else if self.0[0].ident.0 == "Super" {
                a.0[0].ident = path.0[0].clone();

                if path.0.len() > 1 {
                    a.0.splice(
                        1..1,
                        path.0.clone().into_iter().skip(1).map(|v| ExprPathSegment {
                            ident: v,
                            generics: None,
                        }),
                    );
                }
            }
        }

        a.get_rust()
    }
}

impl GetRust for ExprPathSegment {
    fn get_rust(&self) -> String {
        format!(
            "{}{}",
            self.ident.get_rust(),
            self.generics
                .as_ref()
                .map(|v| format!("::{}", v.get_rust()))
                .unwrap_or_default()
        )
    }
}

impl GenRust for Literal {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        match self {
            Self::Int(n) => cg.add(&n.to_string()),
            Self::Float(n) => cg.add(&format!("{n:?}")),
            Self::Bool(b) => cg.add(&b.to_string()),
            Self::String(s) => cg.add(&format!("\"{s}\"")),

            Self::Tuple(values) => {
                cg.add("(");

                for (i, val) in values.iter().enumerate() {
                    if i > 0 {
                        cg.add(", ");
                    }

                    val.gen_rust(ctx, cg);
                }

                cg.add(")");
            }
        }
    }
}

impl GenRust for Expression {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        let ensure_semicolon = if ctx.expr_ensure_semicolon {
            ctx.expr_ensure_semicolon = false;

            true
        } else {
            false
        };

        match self {
            Expression::Path(path) => cg.add(&path.get_rust_ctx(ctx)),
            Expression::Literal(literal) => literal.gen_rust(ctx, cg),
            Expression::Statement(stmt) => stmt.gen_rust(ctx, cg),
            Expression::Array(values) => {
                cg.add("[");

                for (i, val) in values.iter().enumerate() {
                    if i > 0 {
                        cg.add(", ");
                    }

                    val.gen_rust(ctx, cg);
                }

                cg.add("]");
            }
            Expression::ArrayRepeat(value, repeat) => {
                cg.add("[");
                value.gen_rust(ctx, cg);
                cg.add("; ");
                repeat.gen_rust(ctx, cg);
                cg.add("]");
            }
            Expression::Fix {
                initial,
                prefixes,
                postfixes,
            } => {
                prefixes.gen_rust(ctx, cg);
                initial.gen_rust(ctx, cg);
                for postfix in postfixes {
                    postfix.gen_rust(ctx, cg);
                }
            }
            // Safely integrated to handle the tree structure built by the Pratt Parser
            Expression::Binary { lhs, op, rhs } => {
                lhs.gen_rust(ctx, cg);
                cg.add(op);
                rhs.gen_rust(ctx, cg);
            }

            Expression::Closure {
                return_type,
                params,
                body,
            } => {
                cg.add("|");
                for (i, arg) in params.iter().enumerate() {
                    if i > 0 {
                        cg.add(", ");
                    }

                    arg.gen_rust(ctx, cg);
                }
                cg.add("| ");

                if let Some(ty) = return_type {
                    cg.add("-> ");
                    cg.add(&ty.get_rust());
                    cg.add(" ");

                    cg.ensure_brackets_expr(ctx, body);
                } else {
                    body.gen_rust(ctx, cg);
                }
            }
        }

        if ensure_semicolon {
            ctx.expr_ensure_semicolon = true;

            if !self.is_block() {
                cg.add(";");
            }
        }
    }
}

impl GenRust for Prefix {
    fn gen_rust(&self, _ctx: &mut Context, cg: &mut RustCodegen) {
        match self {
            Self::Deref => cg.add("*"),
            Self::Ref => cg.add("&"),
            Self::RefMut => cg.add("&mut "),
            Self::Not => cg.add("!"),
            Self::Neg => cg.add("-"),
        }
    }
}

impl GenRust for Vec<Prefix> {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        for prefix in self {
            prefix.gen_rust(ctx, cg);
        }
    }
}

impl GenRust for Postfix {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        match self {
            Postfix::TupleFieldAccess(field, generics) => cg.add(&format!(
                ".{}{}",
                field,
                generics
                    .iter()
                    .map(|v| format!("::{}", v.get_rust()))
                    .collect::<String>()
            )),
            Postfix::FieldAccess(field, generics) => cg.add(&format!(
                ".{}{}",
                field.get_rust(),
                generics
                    .iter()
                    .map(|v| format!("::{}", v.get_rust()))
                    .collect::<String>()
            )),

            Postfix::Call(args) => {
                cg.add("(");

                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        cg.add(", ");
                    }

                    arg.gen_rust(ctx, cg);
                }

                cg.add(")");
            }

            Postfix::MacroCall { inner, delimiter } => {
                let (open, close) = match delimiter {
                    MacroDelimiter::Paren => ("!(", ")"),
                    MacroDelimiter::Bracket => ("![", "]"),
                    MacroDelimiter::Brace => ("!{", "}"),
                };
                cg.add(open);
                cg.add(inner);
                cg.add(close);
            }

            Postfix::StructCall(fields) => {
                cg.add("{");

                for (name, expr) in fields {
                    cg.add(&name.get_rust());
                    if let Some(expr) = expr {
                        cg.add(": ");
                        expr.gen_rust(ctx, cg);
                    }
                    cg.add(",");
                }

                cg.add("}");
            }

            Postfix::Index(idx) => {
                cg.add("[");
                idx.gen_rust(ctx, cg);
                cg.add("]");
            }

            Postfix::As(ty) => {
                cg.add(" as ");
                cg.add(&ty.get_rust());
            }

            Postfix::Try => cg.add("?"),

            Postfix::Assign(cmp, expr) => {
                cg.add(" ");
                cg.add(cmp);
                expr.gen_rust(ctx, cg);
            }
            Postfix::Increment => cg.add("+=1"),
            Postfix::Decrement => cg.add("-=1"),
        }
    }
}

impl GetRust for Generics {
    fn get_rust(&self) -> String {
        if self.0.len() == 0 {
            String::new()
        } else {
            format!(
                "<{}>",
                self.0
                    .iter()
                    .map(Generic::get_rust)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

impl GetRust for Generic {
    fn get_rust(&self) -> String {
        match self {
            Self::Lifetime(name) => format!("'{}", name.get_rust()),
            Self::Type(ty) => ty.get_rust(),
        }
    }
}
