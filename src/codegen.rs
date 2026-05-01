use parser::ast::{BinaryOp, Block, Expression, Postfix, Statement, TopLevel, TypeExpr};

pub struct RustCodegen {
    output: String,
    indent: usize,
}

impl RustCodegen {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }

    fn indent_str(&self) -> String {
        "    ".repeat(self.indent)
    }

    fn add(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn addln(&mut self, s: &str) {
        self.add(s);
        self.add("\n");
    }

    fn add_indentedln(&mut self, s: &str) {
        self.add(&format!("{}{}\n", self.indent_str(), s));
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
                let path = path.replace("\"", "");
                self.addln(&format!("use {};", path));
            }

            TopLevel::StructDecl {
                export,
                name,
                fields,
            } => {
                let vis = if *export { "pub " } else { "" };

                self.addln(&format!("{}struct {} {{", vis, name));
                self.indent += 1;

                for (field_name, (_, ty)) in &fields.0 {
                    let ty = self.translate_type(ty);
                    self.add_indentedln(&format!("pub {}: {},", field_name, ty));
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
                let vis = if *export { "pub " } else { "" };

                let params_str = params
                    .0
                    .iter()
                    .map(|(n, (_, t))| format!("{}: {}", n, self.translate_type(t)))
                    .collect::<Vec<_>>()
                    .join(", ");

                let ret = return_type
                    .as_ref()
                    .map(|t| format!(" -> {}", self.translate_type(t)))
                    .unwrap_or_default();

                self.addln(&format!("{}fn {}({}){} {{", vis, name, params_str, ret));

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
                "int" => "i32".into(),
                "float" | "float64" => "f64".into(),
                "float32" => "f32".into(),
                "bool" => "bool".into(),
                "string" => "String".into(),
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
                self.add_indentedln("}");
            }

            Statement::VarDecl {
                mutable,
                name,
                init,
                type_,
            } => {
                let mutability = if *mutable { "mut " } else { "" };

                let ty = type_
                    .as_ref()
                    .map(|t| format!(": {}", self.translate_type(t)))
                    .unwrap_or_default();

                let init = init
                    .as_ref()
                    .map(|e| format!(" = {}", self.generate_expression(e)))
                    .unwrap_or_default();

                self.add_indentedln(&format!("let {}{}{}{};", mutability, name, ty, init));
            }

            Statement::VarAssign { target, value } => {
                self.add_indentedln(&format!(
                    "{} = {};",
                    self.generate_expression(target),
                    self.generate_expression(value)
                ));
            }

            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.add_indentedln(&format!("if {} {{", self.generate_expression(condition)));

                self.indent += 1;
                self.generate_statement(then_branch);
                self.indent -= 1;

                self.add_indentedln("}");

                if let Some(else_br) = else_branch {
                    self.add_indentedln("else {");
                    self.indent += 1;
                    self.generate_statement(else_br);
                    self.indent -= 1;
                    self.add_indentedln("}");
                }
            }

            Statement::While { condition, body } => {
                self.add_indentedln(&format!("while {} {{", self.generate_expression(condition)));

                self.indent += 1;
                self.generate_statement(body);
                self.indent -= 1;

                self.add_indentedln("}");
            }

            Statement::For { .. } => {
                // Rust doesn't support C-style for loops
                self.add_indentedln("// TODO: transform into iterator-based loop");
            }

            Statement::Return(expr) => {
                let val = expr
                    .as_ref()
                    .map(|e| self.generate_expression(e))
                    .unwrap_or_default();

                self.add_indentedln(&format!("return {};", val));
            }

            Statement::Break => self.add_indentedln("break;"),
            Statement::Continue => self.add_indentedln("continue;"),
        }
    }

    fn generate_expression(&self, expr: &Expression) -> String {
        match expr {
            Expression::Identifier(name) => name.clone(),
            Expression::IntLiteral(n) => n.to_string(),
            Expression::FloatLiteral(n) => n.to_string(),
            Expression::BoolLiteral(b) => b.to_string(),
            Expression::StringLiteral(s) => format!("\"{}\".to_string()", s),

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
                    let args = args
                        .iter()
                        .map(|a| self.generate_expression(a))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{}({})", result, args)
                }

                Postfix::StructCall(fields) => {
                    let fields = fields
                        .iter()
                        .map(|(k, v)| format!("{}: {}", k, self.generate_expression(v)))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{} {{ {} }}", result, fields)
                }

                Postfix::Index(idx) => {
                    format!("{}[{}]", result, self.generate_expression(idx))
                }

                Postfix::Binary(op, rhs) => {
                    let op = match op {
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

                    format!("{} {} {}", result, op, self.generate_expression(rhs))
                }
            };
        }

        result
    }
}

impl Default for RustCodegen {
    fn default() -> Self {
        Self::new()
    }
}
