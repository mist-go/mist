use mist_parser::ast::*;

use crate::{Context, get_mutable};

use crate::{GenRust, GetRust, RustCodegen};

impl GenRust for Block {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        for stmt in &self.0 {
            cg.add_indented("");
            stmt.gen_rust(ctx, cg);
        }

        if let Some(soft_return) = &self.1 {
            ctx.expr_ensure_semicolon = false;
            soft_return.gen_rust(ctx, cg);
            ctx.expr_ensure_semicolon = true;
        }
    }
}

impl GenRust for Statement {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        match self {
            Statement::Block(block) => {
                cg.add_indentedln("{");
                cg.indent += 1;
                block.gen_rust(ctx, cg);
                cg.indent -= 1;
                cg.add_indentedln("}");
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

                for (pat, body) in match_items {
                    for (i, p) in pat.iter().enumerate() {
                        if i > 0 {
                            cg.add(" | ");
                        }

                        p.gen_rust(ctx, cg);
                    }

                    cg.add("=> {");
                    cg.indent += 1;

                    body.gen_rust(ctx, cg);

                    cg.indent -= 1;
                    cg.add_indentedln("}");
                }

                cg.indent -= 1;
                cg.add_indentedln("}");
            }

            Statement::If {
                initial,
                else_if,
                else_branch,
            } => {
                cg.add("if ");
                initial.condition.gen_rust(ctx, cg);
                cg.ensure_brackets(ctx, &initial.body);

                for else_if_branch in else_if {
                    cg.add("else if");
                    else_if_branch.condition.gen_rust(ctx, cg);
                    cg.ensure_brackets(ctx, &else_if_branch.body);
                }

                if let Some(else_br) = else_branch {
                    cg.add("else");
                    cg.ensure_brackets(ctx, else_br);
                }
            }

            Statement::While(StatementBranch { condition, body }) => {
                cg.add("while ");
                condition.gen_rust(ctx, cg);
                cg.ensure_brackets(ctx, body);
            }

            Statement::CStyleFor {
                init,
                condition,
                update,
                body,
            } => {
                cg.add_indentedln("{");
                cg.indent += 1;

                init.gen_rust(ctx, cg);

                cg.add("while ");

                condition.gen_rust(ctx, cg);

                cg.add("{");

                cg.indent += 1;

                cg.ensure_brackets(ctx, body);

                update.gen_rust(ctx, cg);

                cg.indent -= 1;
                cg.add_indentedln("}");

                cg.indent -= 1;
                cg.add_indentedln("}");
            }

            Statement::For {
                mutable,
                pattern,
                iterator,
                body,
            } => {
                cg.add("for ");
                cg.add(&get_mutable(*mutable));
                pattern.gen_rust(ctx, cg);
                cg.add(" in ");
                iterator.gen_rust(ctx, cg);
                cg.ensure_brackets(ctx, body);
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
        cg.add(&get_mutable(self.mutable));

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
