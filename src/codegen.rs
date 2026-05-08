use parser::ast::{
    Attribute, BinaryOp, Block, EnumItem, Expression, FunctionDecl, Identifier, Literal, Path,
    Pattern, Postfix, Prefix, Statement, StatementBranch, TopLevel, TopLevelKind, TypeExpr,
    TypeExprKind, TypePostfix, VarAssignStmt, VarDecl, VarDeclStmt, Visibility,
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

    pub fn ensure_brackets(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Block(_) => stmt.to_rust(self),
            _ => {
                self.add_indentedln("{");
                self.indent += 1;
                stmt.to_rust(self);
                self.indent -= 1;
                self.add_indentedln("}");
            }
        }
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
            } => {
                prefixes.get_rust()
                    + &initial.get_rust()
                    + &Some(prefixes).get_rust()
                    + &postfixes.get_rust()
            }
        }
    }
}

impl GetRust for Prefix {
    fn get_rust(&self) -> String {
        match self {
            Self::Deref => "*",
            Self::Ref => "&",
            Self::RefMut => "&mut ",
            Self::Not => "!",
            Self::New => "",
        }
        .to_string()
    }
}

impl GetRust for [Prefix] {
    fn get_rust(&self) -> String {
        self.iter().map(Prefix::get_rust).collect()
    }
}

impl GetRust for Option<&Vec<Prefix>> {
    fn get_rust(&self) -> String {
        self.map(|prefixes| {
            prefixes
                .iter()
                .last()
                .map(|p| match p {
                    Prefix::New => "::new",
                    _ => "",
                })
                .unwrap_or_default()
                .to_string()
        })
        .unwrap_or_default()
    }
}

impl GetRust for Postfix {
    fn get_rust(&self) -> String {
        match self {
            Postfix::FieldAccess(field) => format!(".{}", field.get_rust()),

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
                    .map(|(k, v)| format!("{}: {}", k.get_rust(), v.get_rust()))
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
                    BinaryOp::And => "&&",
                    BinaryOp::Or => "||",
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
            Self::Import(path) => cg.addln(&format!("use {};", path.get_rust())),
            Self::Mod(id) => cg.addln(&format!("mod {};", id.get_rust())),
            Self::FunctionDecl(decl) => decl.to_rust(cg),
            Self::StructDecl {
                visibility,
                name,
                fields,
            } => {
                cg.addln(&format!(
                    "{}struct {} {{",
                    visibility.get_rust(),
                    name.get_rust()
                ));
                cg.indent += 1;

                for (field_name, _, ty) in &fields.0 {
                    let ty = ty.get_rust();
                    cg.add_indentedln(&format!("pub {}: {},", field_name.get_rust(), ty));
                }

                cg.indent -= 1;
                cg.addln("}\n");
            }
            Self::EnumDecl {
                visibility,
                name,
                fields,
            } => {
                cg.addln(&format!(
                    "{}enum {} {{",
                    visibility.get_rust(),
                    name.get_rust()
                ));
                cg.indent += 1;

                for field in fields {
                    cg.add_indentedln(&(format!("{}", field.get_rust()) + ","));
                }

                cg.indent -= 1;
                cg.addln("}\n");
            }
            Self::ClassDecl {
                visibility,
                name,
                fields,
                constructor,
                methods,
            } => {
                // Struct decl
                cg.addln(&format!(
                    "{}struct {} {{",
                    visibility.get_rust(),
                    name.get_rust()
                ));
                cg.indent += 1;

                for field in fields {
                    let ty = field.decl.type_.clone().unwrap().get_rust();
                    cg.add_indentedln(&format!("pub {}: {},", field.decl.name.get_rust(), ty));
                }

                cg.indent -= 1;
                cg.addln("}\n");

                // Constructor
                cg.addln(&format!("impl {} {{", name.get_rust()));
                cg.indent += 1;

                let params_str = constructor
                    .params
                    .0
                    .iter()
                    .map(VarDecl::get_rust)
                    .collect::<Vec<_>>()
                    .join(", ");

                cg.add_indentedln(&format!(
                    "{}fn new({}) -> Self {{",
                    constructor.visibility.get_rust(),
                    params_str
                ));
                cg.indent += 1;

                cg.add_indentedln("let mut this: Self = unsafe { std::mem::MaybeUninit::<Self>::zeroed().assume_init() };");

                for field in fields {
                    if let Some(init) = &field.init {
                        cg.add_indentedln(&format!(
                            "this.{} = {};",
                            field.decl.name.get_rust(),
                            init.get_rust()
                        ));
                    }
                }

                cg.add_indentedln(&format!(
                    "this.construct_class({});",
                    constructor
                        .params
                        .0
                        .iter()
                        .map(|e| e.name.get_rust())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));

                cg.add_indentedln("this");

                cg.indent -= 1;
                cg.add_indentedln("}\n");

                // Constructor function
                cg.add_indentedln(&format!(
                    "{}fn construct_class(&mut self, {}) {{",
                    constructor.visibility.get_rust(),
                    params_str
                ));
                cg.indent += 1;

                constructor.body.to_rust(cg);

                cg.indent -= 1;
                cg.add_indentedln("}\n");

                for method in methods {
                    method.to_rust(cg);
                }

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

            Statement::Match(expr, match_items) => {
                cg.add_indentedln(&format!("match {} {{", expr.get_rust()));
                cg.indent += 1;

                for itm in match_items {
                    cg.add_indentedln(&format!("{} =>", itm.0.get_rust()));
                    cg.add_indentedln("{");
                    cg.indent += 1;
                    itm.1.to_rust(cg);
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
                cg.add_indentedln(&format!("if {}", initial.condition.get_rust()));
                cg.ensure_brackets(&initial.body);

                for else_if_branch in else_if {
                    cg.add_indentedln(&format!("else if {}", else_if_branch.condition.get_rust()));
                    cg.ensure_brackets(&else_if_branch.body);
                }

                if let Some(else_br) = else_branch {
                    cg.add_indentedln("else");
                    cg.ensure_brackets(else_br);
                }
            }

            Statement::While(StatementBranch { condition, body }) => {
                cg.add_indentedln(&format!("while {}", condition.get_rust()));
                cg.ensure_brackets(body);
            }

            Statement::CStyleFor {
                init,
                condition,
                update,
                body,
            } => {
                cg.add_indentedln("{");
                cg.indent += 1;

                init.to_rust(cg);

                cg.add_indentedln(&format!("while {}", condition.get_rust()));

                cg.add_indentedln("{");
                cg.indent += 1;

                cg.ensure_brackets(body);

                update.to_rust(cg);

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
                cg.add_indentedln(&format!(
                    "for {}{} in {}",
                    get_mutable(*mutable),
                    pattern.get_rust(),
                    iterator.get_rust()
                ));
                cg.ensure_brackets(body);
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

impl ToRust for FunctionDecl {
    fn to_rust(&self, cg: &mut RustCodegen) {
        let params_str = self
            .params
            .0
            .iter()
            .map(VarDecl::get_rust)
            .collect::<Vec<_>>()
            .join(", ");

        cg.add_indentedln(&format!(
            "{}fn {}({}) -> {} {{",
            self.visibility.get_rust(),
            self.name.get_rust(),
            params_str,
            self.return_type.get_rust()
        ));
        cg.indent += 1;
        self.body.to_rust(cg);
        cg.indent -= 1;
        cg.add_indentedln("}\n");
    }
}

impl GetRust for VarDecl {
    fn get_rust(&self) -> String {
        let ty = self
            .type_
            .as_ref()
            .map(|t| format!(": {}", t.get_rust()))
            .unwrap_or_default();

        format!(
            "{}{}{}",
            get_mutable(self.mutable),
            self.name.get_rust(),
            ty
        )
    }
}

impl GetRust for Path {
    fn get_rust(&self) -> String {
        self.0
            .iter()
            .map(Identifier::get_rust)
            .collect::<Vec<String>>()
            .join("::")
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

impl GetRust for Visibility {
    fn get_rust(&self) -> String {
        match self {
            Visibility::Public => "pub ",
            Visibility::Private => "",
        }
        .to_string()
    }
}

impl GetRust for Identifier {
    fn get_rust(&self) -> String {
        self.0.clone()
    }
}

impl GetRust for EnumItem {
    fn get_rust(&self) -> String {
        match self {
            Self::Named(id) => id.get_rust(),
            Self::Struct(id, s) => format!(
                "{} {{{}}}",
                id.get_rust(),
                s.0.iter()
                    .map(|(id, _, ty)| format!("{}: {}", id.get_rust(), ty.get_rust()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Tuple(id, t) => format!(
                "{} ({})",
                id.get_rust(),
                t.iter()
                    .map(TypeExpr::get_rust)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl GetRust for Pattern {
    fn get_rust(&self) -> String {
        match self {
            Self::Id(id) => id.get_rust(),
            Self::Path(path) => path.get_rust(),
            Self::Literal(lit) => lit.get_rust(),
            Self::Struct(path, ids) => format!(
                "{} {{{}}}",
                path.get_rust(),
                ids.iter()
                    .map(Identifier::get_rust)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Tuple(ids) => format!(
                "({})",
                ids.iter()
                    .map(Identifier::get_rust)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::NamedTuple(path, ids) => {
                format!(
                    "{} ({})",
                    path.get_rust(),
                    ids.iter()
                        .map(Identifier::get_rust)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
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

pub fn get_mutable(mutable: bool) -> String {
    if mutable { "mut " } else { "" }.to_string()
}
