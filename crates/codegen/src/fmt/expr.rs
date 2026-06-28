use mist_parser::ast::*;

use crate::fmt::{Context, GenMist, GetMist, MistCodegen};

impl GenMist for Expression {
    fn gen_mist(&self, ctx: &mut Context, cg: &mut MistCodegen) {
        let ensure_semicolon = if ctx.expr_ensure_semicolon {
            ctx.expr_ensure_semicolon = false;
            true
        } else {
            false
        };

        match self {
            Expression::Path(path) => cg.add(&path.get_mist()),
            Expression::Literal(literal) => literal.gen_mist(ctx, cg),
            Expression::Statement(stmt) => stmt.gen_mist(ctx, cg),
            Expression::Array(values) => {
                cg.add("[");
                for (i, val) in values.iter().enumerate() {
                    if i > 0 {
                        cg.add(", ");
                    }
                    val.gen_mist(ctx, cg);
                }
                cg.add("]");
            }
            Expression::ArrayRepeat(value, repeat) => {
                cg.add("[");
                value.gen_mist(ctx, cg);
                cg.add("; ");
                repeat.gen_mist(ctx, cg);
                cg.add("]");
            }
            Expression::Fix {
                initial,
                prefixes,
                postfixes,
            } => {
                for prefix in prefixes {
                    prefix.gen_mist(ctx, cg);
                }
                initial.gen_mist(ctx, cg);
                for postfix in postfixes {
                    postfix.gen_mist(ctx, cg);
                }
            }
            Expression::Binary { lhs, op, rhs } => {
                lhs.gen_mist(ctx, cg);
                cg.add(" ");
                cg.add(op);
                cg.add(" ");
                rhs.gen_mist(ctx, cg);
            }
            Expression::Closure {
                return_type,
                params,
                body,
            } => {
                cg.add("(");
                for (i, arg) in params.iter().enumerate() {
                    if i > 0 {
                        cg.add(", ");
                    }
                    arg.gen_mist(ctx, cg);
                }
                cg.add(") => ");

                if let Some(ty) = return_type {
                    cg.add(&ty.get_mist());
                    cg.add(" ");
                    cg.ensure_brackets_expr(ctx, body);
                } else {
                    body.gen_mist(ctx, cg);
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

impl GenMist for Prefix {
    fn gen_mist(&self, _ctx: &mut Context, cg: &mut MistCodegen) {
        match self {
            Self::Deref => cg.add("*"),
            Self::Ref => cg.add("&"),
            Self::RefMut => cg.add("&mut "),
            Self::Not => cg.add("!"),
            Self::Neg => cg.add("-"),
        }
    }
}

impl GenMist for Postfix {
    fn gen_mist(&self, ctx: &mut Context, cg: &mut MistCodegen) {
        match self {
            Postfix::TupleFieldAccess(field, generics) => {
                cg.add(&format!(
                    ".{}{}",
                    field,
                    generics
                        .iter()
                        .map(|v| format!("::{}", v.get_mist()))
                        .collect::<String>()
                ));
            }
            Postfix::FieldAccess(field, generics) => {
                cg.add(&format!(
                    ".{}{}",
                    field.get_mist(),
                    generics
                        .iter()
                        .map(|v| format!("::{}", v.get_mist()))
                        .collect::<String>()
                ));
            }
            Postfix::Call(args) => {
                cg.add("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        cg.add(", ");
                    }
                    arg.gen_mist(ctx, cg);
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
                cg.addln(" {");
                cg.indent += 1;
                for (name, expr) in fields {
                    cg.add_indented(&name.get_mist());
                    if let Some(expr) = expr {
                        cg.add(": ");
                        expr.gen_mist(ctx, cg);
                    }
                    cg.addln(",");
                }
                cg.indent -= 1;
                cg.add_indented("}");
            }
            Postfix::Index(idx) => {
                cg.add("[");
                idx.gen_mist(ctx, cg);
                cg.add("]");
            }
            Postfix::As(ty) => {
                cg.add(" as ");
                cg.add(&ty.get_mist());
            }
            Postfix::Try => cg.add("?"),
            Postfix::Increment => cg.add("++"),
            Postfix::Decrement => cg.add("--"),
        }
    }
}
