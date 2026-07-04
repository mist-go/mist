use mist_parser::ast::*;

use crate::{Context, GenSpanTranslation};

use crate::{GenRust, GetRust, RustCodegen};

impl GenRust for Block {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        cg.addln("{");
        cg.indent += 1;

        for stmt in &self.statements {
            cg.add_indented("");
            stmt.gen_rust(ctx, cg);
            if stmt.item.is_semicolon_required() {
                cg.add(";");
            }
            cg.addln("");
        }

        if let Some(soft_return) = &self.soft_return {
            cg.add_indented("");
            soft_return.gen_rust(ctx, cg);
            cg.addln("");
        }

        cg.indent -= 1;
        cg.add_indented("}");
    }
}

impl GenRust for Statement {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        match self {
            Statement::TopLevel(tl) => tl.gen_rust(ctx, cg),
            Statement::Block(block) => block.gen_rust(ctx, cg),

            Statement::UnsafeBlock(block) => {
                cg.add("unsafe ");
                block.gen_rust(ctx, cg);
            }

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
                    match_item.gen_span(cg);

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
                initial.body.gen_rust(ctx, cg);

                for else_if_branch in else_if {
                    cg.add(" else if ");
                    else_if_branch.condition.gen_rust(ctx, cg);
                    cg.add(" ");
                    else_if_branch.body.gen_rust(ctx, cg);
                }

                if let Some(else_br) = else_branch {
                    cg.add(" else ");
                    else_br.gen_rust(ctx, cg);
                }
            }

            Statement::While(StatementBranch { condition, body }) => {
                cg.add("while ");
                condition.gen_rust(ctx, cg);
                cg.add(" ");
                body.gen_rust(ctx, cg);
            }

            Statement::Loop(body) => {
                cg.add("loop ");
                body.gen_rust(ctx, cg);
            }

            Statement::CStyleFor {
                init,
                condition,
                update,
                body,
            } => {
                cg.addln("{");
                cg.indent += 1;

                cg.add_indented("");

                init.gen_rust(ctx, cg);

                if init.is_semicolon_required() {
                    cg.add(";");
                }

                cg.addln("");

                cg.add_indented("while ");

                condition.gen_rust(ctx, cg);

                cg.add(" ");

                cg.add("{");
                cg.indent += 1;

                body.gen_rust(ctx, cg);

                update.gen_rust(ctx, cg);

                if update.is_semicolon_required() {
                    cg.add(";");
                }

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
                body.gen_rust(ctx, cg);
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
        if self.tuple_names.is_empty() {
            self.name.gen_rust(ctx, cg);
            cg.add(
                &self
                    .type_
                    .as_ref()
                    .map(|t| format!(": {}", t.get_rust()))
                    .unwrap_or_default(),
            );
        } else {
            cg.add("(");
            self.name.gen_rust(ctx, cg);
            for name in &self.tuple_names {
                cg.add(", ");

                name.gen_rust(ctx, cg);
            }
            cg.add(")");

            cg.add(
                &self
                    .type_
                    .as_ref()
                    .map(|t| {
                        let t = vec![t.get_rust(); self.tuple_names.len() + 1];
                        format!(": ({})", t.join(", "))
                    })
                    .unwrap_or_default(),
            );
        }
    }
}
