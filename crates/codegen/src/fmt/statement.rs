use mist_parser::ast::*;

use crate::fmt::{Context, GenMist, GetMist, MistCodegen};

impl GenMist for Block {
    fn gen_mist(&self, ctx: &mut Context, cg: &mut MistCodegen) {
        cg.addln("{");
        cg.indent += 1;

        for stmt in &self.statements {
            ctx.expr_ensure_semicolon = true;
            cg.add_indented("");
            stmt.gen_mist(ctx, cg);
            cg.addln("");
        }

        if let Some(soft_return) = &self.soft_return {
            ctx.expr_ensure_semicolon = false;
            cg.add_indented("");
            soft_return.gen_mist(ctx, cg);
            cg.addln("");
        }

        cg.indent -= 1;
        cg.add_indented("}");
    }
}

impl GenMist for Statement {
    fn gen_mist(&self, ctx: &mut Context, cg: &mut MistCodegen) {
        match self {
            Statement::UnsafeBlock(block) => {
                cg.add("unsafe ");
                block.gen_mist(ctx, cg);
            }
            Statement::Block(block) => block.gen_mist(ctx, cg),
            Statement::VarDecl(VarDeclStmt { decl, init }) => {
                if decl.type_.is_some() {
                    cg.add(&decl.type_.as_ref().unwrap().get_mist());
                    cg.add(" ");
                    decl.name.gen_mist(ctx, cg);
                } else {
                    cg.add("let ");
                    decl.name.gen_mist(ctx, cg);
                }
                if let Some(init) = init {
                    cg.add(" = ");
                    init.gen_mist(ctx, cg);
                }
            }
            Statement::Match(expr, match_items) => {
                cg.add("match ");
                expr.gen_mist(ctx, cg);
                cg.add(" {");
                if !match_items.is_empty() {
                    cg.addln("");
                    cg.indent += 1;
                    for match_item in match_items {
                        let MatchItem(pat, body) = &match_item.item;
                        cg.add_indented("");
                        for (i, p) in pat.iter().enumerate() {
                            if i > 0 {
                                cg.add(" | ");
                            }
                            p.gen_mist(ctx, cg);
                        }
                        cg.add(" => ");
                        if body.is_block() {
                            body.gen_mist(ctx, cg);
                            cg.addln("");
                        } else {
                            body.gen_mist(ctx, cg);
                            cg.addln(",");
                        }
                    }
                    cg.indent -= 1;
                    cg.add_indented("}");
                } else {
                    cg.add("}");
                }
            }
            Statement::If {
                initial,
                else_if,
                else_branch,
            } => {
                cg.add("if ");
                ctx.expr_ensure_semicolon = false;
                initial.condition.gen_mist(ctx, cg);
                cg.add(" ");
                initial.body.gen_mist(ctx, cg);

                for else_if_branch in else_if {
                    cg.add(" else if ");
                    ctx.expr_ensure_semicolon = false;
                    else_if_branch.condition.gen_mist(ctx, cg);
                    cg.add(" ");
                    else_if_branch.body.gen_mist(ctx, cg);
                }

                if let Some(else_br) = else_branch {
                    cg.add(" else ");
                    else_br.gen_mist(ctx, cg);
                }
            }
            Statement::While(StatementBranch { condition, body }) => {
                cg.add("while ");
                condition.gen_mist(ctx, cg);
                cg.add(" ");
                body.gen_mist(ctx, cg);
            }
            Statement::Loop(body) => {
                cg.add("loop ");
                body.gen_mist(ctx, cg);
            }
            Statement::CStyleFor {
                init,
                condition,
                update,
                body,
            } => {
                cg.add("for (");
                ctx.expr_ensure_semicolon = true;
                init.gen_mist(ctx, cg);
                cg.add(" ");
                ctx.expr_ensure_semicolon = false;
                condition.gen_mist(ctx, cg);
                cg.add(" ");
                update.gen_mist(ctx, cg);
                cg.add(") ");
                body.gen_mist(ctx, cg);
            }
            Statement::For {
                pattern,
                iterator,
                body,
            } => {
                cg.add("for ");
                pattern.gen_mist(ctx, cg);
                cg.add(" in ");
                iterator.gen_mist(ctx, cg);
                cg.add(" ");
                body.gen_mist(ctx, cg);
            }
            Statement::Return(expr) => {
                cg.add("return");
                if let Some(expr) = expr {
                    cg.add(" ");
                    expr.gen_mist(ctx, cg);
                }
            }
            Statement::Break => cg.add("break"),
            Statement::Continue => cg.add("continue"),
        }
    }
}
