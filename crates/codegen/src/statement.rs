use mist_parser::ast::*;

use crate::Context;

use crate::{GenRust, GetRust, RustCodegen};

impl GenRust for Block {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        cg.addln("{");
        cg.indent += 1;

        for stmt in &self.0 {
            ctx.expr_ensure_semicolon = true;
            stmt.gen_rust(ctx, cg);
            cg.addln("");
        }

        if let Some(soft_return) = &self.1 {
            ctx.expr_ensure_semicolon = false;
            soft_return.gen_rust(ctx, cg);
            cg.addln("");
        }

        cg.indent -= 1;
        cg.add_indented("}");
    }
}

impl GenRust for StatementBody {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        match self {
            Self::Expression(expr) => expr.gen_rust(ctx, cg),
            Self::Statement(stmt) => {
                ctx.expr_ensure_semicolon = true;
                stmt.gen_rust(ctx, cg);
            }
        }
    }
}

impl GenRust for Statement {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        match self {
            Statement::Block(block) => block.gen_rust(ctx, cg),

            Statement::VarDecl(VarDeclStmt { decl, init }) => {
                cg.add("let ");
                decl.gen_rust(ctx, cg);

                if let Some(init) = init {
                    cg.add(" = ");
                    init.gen_rust(ctx, cg);
                }
            }

            Statement::Match(expr, match_items) => {
                cg.add("match ");
                expr.gen_rust(ctx, cg);
                cg.add(" {");
                cg.indent += 1;

                for match_item in match_items {
                    cg.add_indentedln(&match_item.get_comment());

                    let MatchItem(pat, body) = &match_item.item;

                    for (i, p) in pat.iter().enumerate() {
                        cg.addln("");
                        cg.add_indented("");
                        if i > 0 {
                            cg.add(" | ");
                        }

                        p.gen_rust(ctx, cg);
                    }

                    cg.add(" => ");

                    cg.ensure_brackets_expr(ctx, body);
                }

                cg.indent -= 1;
                cg.addln("");
                cg.add_indented("}");
            }

            Statement::If {
                initial,
                else_if,
                else_branch,
            } => {
                cg.add("if ");
                initial.condition.gen_rust(ctx, cg);
                cg.add(" ");
                cg.ensure_brackets_body(ctx, &initial.body);

                for else_if_branch in else_if {
                    cg.add("else if");
                    else_if_branch.condition.gen_rust(ctx, cg);
                    cg.add(" ");
                    cg.ensure_brackets_body(ctx, &else_if_branch.body);
                }

                if let Some(else_br) = else_branch {
                    cg.add(" else ");
                    cg.ensure_brackets_body(ctx, else_br);
                }
            }

            Statement::While(StatementBranch { condition, body }) => {
                cg.add("while ");
                condition.gen_rust(ctx, cg);
                cg.add(" ");
                cg.ensure_brackets_body(ctx, body);
            }

            Statement::Loop(body) => {
                cg.add("loop ");
                cg.ensure_brackets_body(ctx, body);
            }

            Statement::CStyleFor {
                init,
                condition,
                update,
                body,
            } => {
                cg.addln("{");
                cg.indent += 1;

                ctx.expr_ensure_semicolon = true;

                cg.add_indented("");

                init.gen_rust(ctx, cg);

                cg.addln("");

                cg.add_indented("while ");

                ctx.expr_ensure_semicolon = false;
                condition.gen_rust(ctx, cg);

                cg.add(" ");

                cg.add("{");
                cg.indent += 1;

                ctx.expr_ensure_semicolon = true;

                body.gen_rust(ctx, cg);

                update.gen_rust(ctx, cg);

                cg.addln("");

                cg.indent -= 1;
                cg.add_indentedln("}");

                cg.indent -= 1;
                cg.add_indented("}");
            }

            Statement::For {
                pattern,
                iterator,
                body,
            } => {
                cg.add("for ");
                pattern.gen_rust(ctx, cg);
                cg.add(" in ");
                iterator.gen_rust(ctx, cg);
                cg.ensure_brackets_body(ctx, body);
            }

            Statement::Return(expr) => {
                cg.add("return ");
                if let Some(expr) = expr {
                    expr.gen_rust(ctx, cg);
                }
            }

            Statement::Break => cg.add_indentedln("break"),
            Statement::Continue => cg.add_indentedln("continue"),
        }
    }
}

impl GenRust for VarDecl {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        self.name.gen_rust(ctx, cg);

        cg.add(
            &self
                .type_
                .as_ref()
                .map(|t| format!(": {}", t.get_rust()))
                .unwrap_or_default(),
        );
    }
}
