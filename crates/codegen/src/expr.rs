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

                for val in values {
                    val.gen_rust(ctx, cg);
                }

                cg.add(")");
            }

            Self::Array(values) => {
                cg.add("[");

                for val in values {
                    val.gen_rust(ctx, cg);
                }

                cg.add("]");
            }

            Self::ArrayRepeat(value, repeat) => {
                cg.add("[");
                value.gen_rust(ctx, cg);
                cg.add("; ");
                repeat.gen_rust(ctx, cg);
                cg.add("]");
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
            Expression::Path(path) => cg.add(&path.get_rust()),
            Expression::Literal(literal) => literal.gen_rust(ctx, cg),
            Expression::Statement(stmt) => stmt.gen_rust(ctx, cg),
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
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        match self {
            Self::Deref => cg.add("*"),
            Self::Ref => cg.add("&"),
            Self::RefMut => cg.add("&mut "),
            Self::Not => cg.add("!"),
            Self::Neg => cg.add("-"),
            Self::Closure(ty, args) => {
                cg.add("|");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        cg.add(", ");
                    }

                    arg.gen_rust(ctx, cg);
                }
                cg.add("| ");

                if let Some(ty) = ty {
                    cg.add("-> ");
                    cg.add(&ty.get_rust());
                    cg.add(" ");
                }
            }
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

            Postfix::MacroCall(inner) => {
                cg.add("!(");
                cg.add(inner);
                cg.add(")");
            }

            Postfix::StructCall(fields) => {
                cg.add("{");

                for (name, expr) in fields {
                    cg.add(&name.get_rust());
                    cg.add(": ");
                    expr.gen_rust(ctx, cg);
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
