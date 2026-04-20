use parser::ast::{
    BinaryOp, Block, Expression, Postfix, Statement, TopLevel, TypeExpr, VarKind,
};

pub struct GoCodegen {
    output: String,
    indent: usize,
}

impl GoCodegen {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }

    fn indent_str(&self) -> String {
        "    ".repeat(self.indent)
    }

    fn add_indented(&mut self, s: &str) {
        self.output.push_str(&format!("{}{}", self.indent_str(), s));
    }

    fn add(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn addln(&mut self, s: &str) {
        self.add(s);
        self.add("\n");
    }

    fn add_indentedln(&mut self, s: &str) {
        self.add_indented(s);
        self.add("\n");
    }

    pub fn generate(&mut self, toplevels: &[TopLevel]) -> String {
        for tl in toplevels {
            self.generate_toplevel(tl);
        }
        self.output.clone()
    }

    fn generate_toplevel(&mut self, tl: &TopLevel) {
        match tl {
            TopLevel::Import(path) => {
                let import_path = path.replace("\"", "");
                if import_path.starts_with("./") || import_path.starts_with("/") {
                    self.addln(&format!("import \"{}\"", import_path));
                } else {
                    self.addln(&format!("import \"{}\"", import_path));
                }
                self.addln("");
            }
            TopLevel::StructDecl { export, name, fields } => {
                let name = if *export { name } else { name };
                self.addln(&format!("type {} struct {{", name));
                self.indent += 1;
                for (field_name, (_, ty)) in &fields.0 {
                    let go_ty = self.translate_type(ty);
                    self.addln(&format!("{} {}", field_name, go_ty));
                }
                self.indent -= 1;
                self.addln("}\n");
            }
            TopLevel::FunctionDecl {
                export,
                name,
                params,
                return_type,
                body,
            } => {
                let name = if *export { format!("{}", name) } else { name.clone() };
                let params_str = params
                    .0
                    .iter()
                    .map(|(n, (_, t))| format!("{} {}", n, self.translate_type(t)))
                    .collect::<Vec<_>>()
                    .join(", ");

                let ret_ty = return_type
                    .as_ref()
                    .map(|t| self.translate_type(t))
                    .unwrap_or_else(|| "".to_string());

                if ret_ty.is_empty() {
                    self.addln(&format!("func {}({}) {{", name, params_str));
                } else {
                    self.addln(&format!("func {}({}) {} {{", name, params_str, ret_ty));
                }
                self.indent += 1;
                self.generate_block(body);
                self.indent -= 1;
                self.addln("}\n");
            }
        }
    }

    fn translate_type(&self, ty: &TypeExpr) -> String {
        match ty {
            TypeExpr::Identifier(name) => match name.as_str() {
                "int" => "int".to_string(),
                "float" | "float64" => "float64".to_string(),
                "float32" => "float32".to_string(),
                "bool" => "bool".to_string(),
                "string" => "string".to_string(),
                "byte" => "byte".to_string(),
                "rune" => "rune".to_string(),
                _ => name.clone(),
            },
        }
    }

    fn generate_block(&mut self, block: &Block) {
        for stmt in &block.0 {
            self.generate_statement(stmt);
        }
    }

    fn generate_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Expression(expr) => {
                self.add_indentedln(&format!("{};", self.generate_expression(expr)));
            }
            Statement::Block(block) => {
                self.add_indentedln("{");
                self.indent += 1;
                self.generate_block(block);
                self.indent -= 1;
                self.add_indentedln("}\n");
            }
            Statement::VarDecl { kind, name, init } => {
                let go_kind = match kind {
                    VarKind::Let | VarKind::Const => "var",
                    VarKind::Var => "var",
                };
                let init_expr = init
                    .as_ref()
                    .map(|e| format!(" = {}", self.generate_expression(e)))
                    .unwrap_or_else(|| "".to_string());
                self.add_indentedln(&format!("{} {}{};\n", go_kind, name, init_expr));
            }
            Statement::VarAssign { target, value } => {
                self.add_indentedln(&format!(
                    "{} = {};\n",
                    self.generate_expression(target),
                    self.generate_expression(value)
                ));
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.add_indented(&format!(
                    "if {} ",
                    self.generate_expression(condition)
                ));
                self.generate_statement(then_branch);
                if let Some(else_br) = else_branch {
                    self.add_indented("else ");
                    self.generate_statement(else_br);
                }
            }
            Statement::While {
                condition,
                body,
            } => {
                self.add_indented(&format!(
                    "for {} ",
                    self.generate_expression(condition)
                ));
                self.generate_statement(body);
            }
            Statement::For {
                init,
                condition,
                update,
                body,
            } => {
                let (kind, init_name, init_val) = init;
                let init_expr = init_val
                    .as_ref()
                    .map(|e| format!(" = {}", self.generate_expression(e)))
                    .unwrap_or_else(|| "".to_string());
                let init_str = format!("{} {}{}", self.var_kind_to_go(kind), init_name, init_expr);

                let cond_str = condition
                    .as_ref()
                    .map(|e| self.generate_expression(e))
                    .unwrap_or_else(|| "true".to_string());

                let update_str = update
                    .as_ref()
                    .map(|s| self.generate_expression(&self.stmt_to_expr(s)))
                    .unwrap_or_else(|| "".to_string());

                self.add_indented(&format!("for {}; {}; {} ", init_str, cond_str, update_str));
                self.generate_statement(body);
            }
            Statement::Return(expr) => {
                let ret_val = expr
                    .as_ref()
                    .map(|e| self.generate_expression(e))
                    .unwrap_or_else(|| "".to_string());
                self.add_indentedln(&format!("return {};\n", ret_val));
            }
            Statement::Break => {
                self.add_indentedln("break;\n");
            }
            Statement::Continue => {
                self.add_indentedln("continue;\n");
            }
        }
    }

    fn stmt_to_expr(&self, stmt: &Statement) -> Expression {
        match stmt {
            Statement::Expression(e) => e.clone(),
            _ => Expression::Identifier(String::new()),
        }
    }

    fn var_kind_to_go(&self, kind: &VarKind) -> String {
        match kind {
            VarKind::Let | VarKind::Const => "var".to_string(),
            VarKind::Var => "var".to_string(),
        }
    }

    fn generate_expression(&self, expr: &Expression) -> String {
        match expr {
            Expression::Identifier(name) => name.clone(),
            Expression::IntLiteral(n) => n.to_string(),
            Expression::FloatLiteral(n) => n.to_string(),
            Expression::BoolLiteral(b) => b.to_string(),
            Expression::StringLiteral(s) => format!("\"{}\"", s),
            Expression::Postfix { initial, postfixes } => {
                let base = self.generate_expression(initial);
                self.apply_postfixes(&base, postfixes)
            }
        }
    }

    fn apply_postfixes(&self, base: &str, postfixes: &[Postfix]) -> String {
        let mut result = base.to_string();
        for postfix in postfixes {
            result = match postfix {
                Postfix::FieldAccess(field) => format!("{}.{}", result, field),
                Postfix::Call(args) => {
                    let args_str = args
                        .iter()
                        .map(|a| self.generate_expression(a))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{}({})", result, args_str)
                }
                Postfix::Index(idx) => format!(
                    "{}[{}]",
                    result,
                    self.generate_expression(idx)
                ),
                Postfix::Binary(op, rhs) => {
                    let op_str = match op {
                        BinaryOp::Plus => "+",
                        BinaryOp::Minus => "-",
                        BinaryOp::Multiply => "*",
                        BinaryOp::Divide => "/",
                        BinaryOp::Modulo => "%",
                        BinaryOp::Equal => "==",
                        BinaryOp::NotEqual => "!=",
                        BinaryOp::LessThan => "<",
                        BinaryOp::GreaterThan => ">",
                        BinaryOp::LessThanOrEqual => "<=",
                        BinaryOp::GreaterThanOrEqual => ">=",
                    };
                    format!(
                        "{} {} {}",
                        result,
                        op_str,
                        self.generate_expression(rhs)
                    )
                }
            };
        }
        result
    }
}

impl Default for GoCodegen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use parser::ast::{ParamList, TypeExpr, Block, Statement, Expression, TopLevel};

    #[test]
    fn test_int_literal() {
        let cg = GoCodegen::new();
        let expr = Expression::IntLiteral(42);
        let result = cg.generate_expression(&expr);
        assert_eq!(result, "42");
    }

    #[test]
    fn test_string_literal() {
        let cg = GoCodegen::new();
        let expr = Expression::StringLiteral("hello".to_string());
        let result = cg.generate_expression(&expr);
        assert_eq!(result, "\"hello\"");
    }

    #[test]
    fn test_function_decl() {
        let mut cg = GoCodegen::new();
        let toplevel = TopLevel::FunctionDecl {
            export: true,
            name: "main".to_string(),
            params: ParamList(HashMap::new()),
            return_type: None,
            body: Block(vec![Statement::Return(None)]),
        };
        cg.generate_toplevel(&toplevel);
        let output = cg.output.clone();
        assert!(output.contains("func main()"));
    }

    #[test]
    fn test_struct_decl() {
        let mut cg = GoCodegen::new();
        let mut fields = HashMap::new();
        fields.insert("x".to_string(), (true, TypeExpr::Identifier("int".to_string())));
        let toplevel = TopLevel::StructDecl {
            export: true,
            name: "Point".to_string(),
            fields: ParamList(fields),
        };
        cg.generate_toplevel(&toplevel);
        let output = cg.output.clone();
        assert!(output.contains("type Point struct"));
        assert!(output.contains("x int"));
    }

    #[test]
    fn test_if_statement() {
        let mut cg = GoCodegen::new();
        let stmt = Statement::If {
            condition: Expression::Identifier("x".to_string()),
            then_branch: Box::new(Statement::Return(Some(Expression::IntLiteral(1)))),
            else_branch: None,
        };
        cg.generate_statement(&stmt);
        let output = cg.output.clone();
        assert!(output.contains("if x"));
    }
}