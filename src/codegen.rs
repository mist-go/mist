use parser::ast::{
    Attribute, BinaryOp, Block, Expression, IfStmt, Literal, Path, Postfix, Prefix, Statement,
    TopLevel, TopLevelKind, TypeExpr, TypeExprKind, TypePostfix, VarAssignStmt, VarDecl,
    VarDeclStmt, WhileStmt,
};

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// Implemented by nodes that *write* into the codegen output buffer.
/// Requires `&mut RustCodegen` because it calls `add` / `addln` / indentation helpers.
pub trait ToRust {
    fn to_rust(&self, cg: &mut RustCodegen);
}

/// Implemented by nodes that *produce* a `String` without mutating the codegen.
/// Only needs `&RustCodegen` (e.g. for indent level or helper access).
pub trait GetRust {
    fn get_rust(&self) -> String;
}

// ---------------------------------------------------------------------------
// Codegen struct
// ---------------------------------------------------------------------------

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
        let line = format!("{}{}\n", self.indent_str(), s);
        self.add(&line);
    }

    pub fn generate(&mut self, toplevels: &[TopLevel]) -> String {
        for tl in toplevels {
            tl.to_rust(self);
        }
        self.output.clone()
    }
}

impl Default for RustCodegen {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// GetRust — pure string production (expressions, types)
// ---------------------------------------------------------------------------

impl GetRust for TypeExpr {
    fn get_rust(&self) -> String {
        get_type_postfixes(&self.1) + &self.0.get_rust()
    }
}

impl GetRust for TypeExprKind {
    fn get_rust(&self) -> String {
        match self {
            TypeExprKind::Path(path) => get_static_type_path(path),
            TypeExprKind::PathParams(path, params) => {
                format!(
                    "{}<{}>",
                    get_static_type_path(path),
                    params
                        .iter()
                        .map(|t| t.get_rust())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            TypeExprKind::Tuple(types) => format!(
                "({})",
                types
                    .iter()
                    .map(|t| t.get_rust())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl GetRust for Literal {
    fn get_rust(&self) -> String {
        match self {
            Self::Int(n) => n.to_string(),
            Self::Float(n) => format!("{n:?}"),
            Self::Bool(b) => b.to_string(),
            Self::String(s) => format!("\"{s}\""),
            Self::Tuple(t) => {
                format!(
                    "({})",
                    t.iter()
                        .map(Expression::get_rust)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

impl GetRust for Expression {
    fn get_rust(&self) -> String {
        match self {
            Expression::Path(path) => path.get_rust(),
            Expression::Literal(literal) => literal.get_rust(),
            Expression::Fix {
                initial,
                prefixes,
                postfixes,
            } => prefixes.get_rust() + &initial.get_rust() + &postfixes.get_rust(),
        }
    }
}

impl GetRust for Prefix {
    fn get_rust(&self) -> String {
        match self {
            Self::Deref => "*",
            Self::Ref => "&",
            Self::RefMut => "&mut ",
        }
        .to_string()
    }
}

impl GetRust for [Prefix] {
    fn get_rust(&self) -> String {
        self.iter().map(Prefix::get_rust).collect()
    }
}

impl GetRust for Postfix {
    fn get_rust(&self) -> String {
        match self {
            Postfix::FieldAccess(field) => format!(".{}", field),

            Postfix::Call(args) => {
                let args = args
                    .iter()
                    .map(|a| a.get_rust())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", args)
            }

            Postfix::MacroCall(inner) => {
                format!("!({})", inner)
            }

            Postfix::StructCall(fields) => {
                let fields = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.get_rust()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{ {} }}", fields)
            }

            Postfix::Index(idx) => {
                format!("[{}]", idx.get_rust())
            }

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
                format!(" {} {}", op_str, rhs.get_rust())
            }
        }
    }
}

impl GetRust for [Postfix] {
    fn get_rust(&self) -> String {
        self.iter().map(Postfix::get_rust).collect()
    }
}

// ---------------------------------------------------------------------------
// ToRust — output-writing (top-level, statements, blocks)
// ---------------------------------------------------------------------------

impl ToRust for Block {
    fn to_rust(&self, cg: &mut RustCodegen) {
        for stmt in &self.0 {
            stmt.to_rust(cg);
        }
    }
}

impl ToRust for TopLevel {
    fn to_rust(&self, cg: &mut RustCodegen) {
        match &self.0 {
            TopLevelKind::ModAttribute => {
                for attr in &self.1 {
                    cg.addln(&format!("#![{}]", attr.get_rust()));
                }
            }
            _ => {
                for attr in &self.1 {
                    cg.addln(&format!("#[{}]", attr.get_rust()));
                }
            }
        }

        self.0.to_rust(cg);
    }
}

impl GetRust for Attribute {
    fn get_rust(&self) -> String {
        match self {
            Self::Path(path) => path.get_rust(),
            Self::NameValue { path, value } => {
                format!("{} = {}", path.get_rust(), value.get_rust())
            }
            Self::List { path, items } => {
                format!(
                    "{}({})",
                    path.get_rust(),
                    items
                        .iter()
                        .map(Attribute::get_rust)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

impl ToRust for TopLevelKind {
    fn to_rust(&self, cg: &mut RustCodegen) {
        match self {
            Self::ModAttribute => {}
            Self::Include(path) => {
                cg.addln(&format!("use {};", path.get_rust()));
            }

            Self::StructDecl {
                export,
                name,
                fields,
            } => {
                let vis = if *export { "pub " } else { "" };
                cg.addln(&format!("{}struct {} {{", vis, name));
                cg.indent += 1;

                for (field_name, (_, ty)) in &fields.0 {
                    let ty = ty.get_rust();
                    cg.add_indentedln(&format!("pub {}: {},", field_name, ty));
                }

                cg.indent -= 1;
                cg.addln("}\n");
            }

            Self::FunctionDecl {
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
                    .map(VarDecl::get_rust)
                    .collect::<Vec<_>>()
                    .join(", ");

                cg.addln(&format!(
                    "{}fn {}({}) -> {} {{",
                    vis,
                    name,
                    params_str,
                    return_type.get_rust()
                ));
                cg.indent += 1;
                body.to_rust(cg);
                cg.indent -= 1;
                cg.addln("}\n");
            }
        }
    }
}

impl ToRust for Statement {
    fn to_rust(&self, cg: &mut RustCodegen) {
        match self {
            Statement::Expression(expr) => {
                cg.add_indentedln(&format!("{};", expr.get_rust()));
            }

            Statement::Block(block) => {
                cg.add_indentedln("{");
                cg.indent += 1;
                block.to_rust(cg);
                cg.indent -= 1;
                cg.add_indentedln("}");
            }

            Statement::VarDecl(VarDeclStmt { decl, init }) => {
                let init = init
                    .as_ref()
                    .map(|e| format!(" = {}", e.get_rust()))
                    .unwrap_or_default();

                cg.add_indentedln(&format!("let {}{};", decl.get_rust(), init));
            }

            Statement::VarAssign(VarAssignStmt { target, value }) => {
                cg.add_indentedln(&format!("{} = {};", target.get_rust(), value.get_rust(),));
            }

            Statement::If(IfStmt {
                condition,
                then_branch,
                else_branch,
            }) => {
                cg.add_indentedln(&format!("if {}", condition.get_rust()));
                then_branch.to_rust(cg);

                if let Some(else_br) = else_branch {
                    cg.add_indentedln("else");
                    else_br.to_rust(cg);
                }
            }

            Statement::While(WhileStmt { condition, body }) => {
                cg.add_indentedln(&format!("while {} {{", condition.get_rust()));
                cg.indent += 1;
                body.to_rust(cg);
                cg.indent -= 1;
                cg.add_indentedln("}");
            }

            Statement::For { .. } => {
                cg.add_indentedln("// TODO: transform into iterator-based loop");
            }

            Statement::Return(expr) => {
                let val = expr.as_ref().map(|e| e.get_rust()).unwrap_or_default();
                cg.add_indentedln(&format!("return {};", val));
            }

            Statement::Break => cg.add_indentedln("break;"),
            Statement::Continue => cg.add_indentedln("continue;"),
        }
    }
}

impl GetRust for VarDecl {
    fn get_rust(&self) -> String {
        let mutability = if self.mutable { "mut " } else { "" };

        let ty = self
            .type_
            .as_ref()
            .map(|t| format!(": {}", t.get_rust()))
            .unwrap_or_default();

        format!("{}{}{}", mutability, self.name, ty)
    }
}

impl GetRust for Path {
    fn get_rust(&self) -> String {
        self.0.join("::")
    }
}

impl GetRust for TypePostfix {
    fn get_rust(&self) -> String {
        match self {
            TypePostfix::Ref => format!("&"),
            TypePostfix::RefMut => format!("&mut "),
        }
    }
}

pub fn get_static_type_path(path: &Path) -> String {
    let rust_path = path.get_rust();

    if rust_path == "void" {
        format!("()")
    } else {
        rust_path
    }
}

pub fn get_type_postfixes(postfixes: &[TypePostfix]) -> String {
    postfixes.iter().map(TypePostfix::get_rust).collect()
}
