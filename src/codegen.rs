use mist_parser::ast::{
    Attribute, Block, ClassItem, EnumItem, ExprPath, ExprPathSegment, Expression, FieldDecl,
    FunctionDecl, Generic, GenericDecl, Generics, GenericsDecl, Identifier, ImplDecl, Literal,
    Path, Pattern, Postfix, Prefix, Spanned, Statement, StatementBranch, TopLevel, TopLevelKind,
    TypeExpr, TypeExprKind, TypePostfix, VarDecl, VarDeclStmt, Visibility,
};

// ---------------------------------------------------------------------------
// Traits
// ---------------------------------------------------------------------------

/// Implemented by nodes that *write* into the codegen output buffer.
/// Requires `&mut RustCodegen` because it calls `add` / `addln` / indentation helpers.
pub trait ToRust {
    fn to_rust(self, cg: &mut RustCodegen);
}

/// Implemented by nodes that *produce* a `String` without mutating the codegen.
/// Only needs `&RustCodegen` (e.g. for indent level or helper access).
pub trait GetRust {
    fn get_rust(self) -> String;
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

    pub fn generate(&mut self, toplevels: Vec<TopLevel>) -> String {
        for tl in toplevels {
            tl.to_rust(self);
        }

        self.output.clone()
    }

    pub fn ensure_brackets(&mut self, stmt: Box<Statement>) {
        match *stmt {
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

impl<T: GetRust> GetRust for Spanned<T> {
    fn get_rust(self) -> String {
        format!(
            "/* {}:{} */ {}",
            self.line,
            self.column,
            self.item.get_rust()
        )
    }
}

impl GetRust for TypeExpr {
    fn get_rust(self) -> String {
        get_type_postfixes(self.1) + &self.0.get_rust()
    }
}

impl GetRust for TypeExprKind {
    fn get_rust(self) -> String {
        match self {
            TypeExprKind::Path(path) => get_static_type_path(path),
            TypeExprKind::Lifetime(name) => format!("'{}", name.get_rust()),
            TypeExprKind::PathParams(path, params) => {
                format!(
                    "{}<{}>",
                    get_static_type_path(path),
                    params
                        .into_iter()
                        .map(|t| t.get_rust())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            TypeExprKind::Tuple(types) => format!(
                "({})",
                types
                    .into_iter()
                    .map(|t| t.get_rust())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl GetRust for Expression {
    fn get_rust(self) -> String {
        match self {
            Expression::Path(path) => path.get_rust(),
            Expression::Literal(literal) => literal.get_rust(),
            Expression::Statement(stmt) => {
                let mut cg = RustCodegen::new();
                cg.indent += 1;
                stmt.to_rust(&mut cg);
                cg.output
            }
            Expression::Fix {
                initial,
                prefixes,
                postfixes,
            } => {
                prefixes.clone().get_rust()
                    + &initial.get_rust()
                    + &Some(prefixes).get_rust()
                    + &postfixes.get_rust()
            }
            // Safely integrated to handle the tree structure built by the Pratt Parser
            Expression::Binary { lhs, op, rhs } => {
                format!("{} {} {}", lhs.get_rust(), op, rhs.get_rust())
            }
        }
    }
}

impl GetRust for Prefix {
    fn get_rust(self) -> String {
        match self {
            Self::Deref => "*",
            Self::Ref => "&",
            Self::RefMut => "&mut ",
            Self::Not => "!",
            Self::New(_) => "",
            Self::Neg => "-",
        }
        .to_string()
    }
}

impl GetRust for Vec<Prefix> {
    fn get_rust(self) -> String {
        self.into_iter().map(Prefix::get_rust).collect()
    }
}

impl GetRust for ExprPath {
    fn get_rust(self) -> String {
        self.0
            .into_iter()
            .map(ExprPathSegment::get_rust)
            .collect::<Vec<_>>()
            .join("::")
    }
}

impl GetRust for ExprPathSegment {
    fn get_rust(self) -> String {
        format!(
            "{}{}",
            self.ident.get_rust(),
            self.generics
                .map(|v| format!("::{}", v.get_rust()))
                .unwrap_or_default()
        )
    }
}

impl GetRust for Option<Vec<Prefix>> {
    fn get_rust(self) -> String {
        self.map(|prefixes| {
            prefixes
                .into_iter()
                .last()
                .map(|p| match p {
                    Prefix::New(generics) => format!(
                        "::new{}",
                        generics
                            .map(|v| format!("::{}", v.get_rust()))
                            .unwrap_or_default(),
                    ),
                    _ => String::new(),
                })
                .unwrap_or_default()
                .to_string()
        })
        .unwrap_or_default()
    }
}

impl GetRust for Postfix {
    fn get_rust(self) -> String {
        match self {
            Postfix::FieldAccess(field, generics) => format!(
                ".{}{}",
                field.get_rust(),
                generics
                    .map(|v| format!("::{}", v.get_rust()))
                    .unwrap_or_default()
            ),

            Postfix::Call(args) => {
                let args = args
                    .into_iter()
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
                    .into_iter()
                    .map(|(k, v)| format!("{}: {}", k.get_rust(), v.get_rust()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{ {} }}", fields)
            }

            Postfix::Index(idx) => {
                format!("[{}]", idx.get_rust())
            }

            Postfix::As(ty) => format!(" as {}", ty.get_rust()),

            Postfix::Try => String::from("?"),

            Postfix::Assign(cmp, expr) => format!("{cmp} {}", expr.get_rust()),
            Postfix::Increment => "+=1".to_string(),
            Postfix::Decrement => "-=1".to_string(),
        }
    }
}

impl GetRust for Vec<Postfix> {
    fn get_rust(self) -> String {
        self.into_iter().map(Postfix::get_rust).collect()
    }
}
// ---------------------------------------------------------------------------
// ToRust — output-writing (top-level, statements, blocks)
// ---------------------------------------------------------------------------

impl<T: ToRust> ToRust for Spanned<T> {
    fn to_rust(self, cg: &mut RustCodegen) {
        cg.add_indentedln(&format!("/* {}:{} */", self.line, self.column));
        self.item.to_rust(cg);
    }
}

impl ToRust for Block {
    fn to_rust(self, cg: &mut RustCodegen) {
        for stmt in self.0 {
            cg.add_indentedln(&(stmt.get_rust() + ";"));
        }

        if let Some(soft_return) = self.1 {
            cg.add_indentedln(&soft_return.get_rust());
        }
    }
}

impl ToRust for TopLevel {
    fn to_rust(self, cg: &mut RustCodegen) {
        if let TopLevelKind::ModAttribute = self.0.item {
            for attr in self.1 {
                cg.addln(&format!("#![{}]", attr.get_rust()));
            }
        } else {
            for attr in self.1 {
                cg.addln(&format!("#[{}]", attr.get_rust()));
            }
        }

        self.0.to_rust(cg);
    }
}

impl GetRust for Attribute {
    fn get_rust(self) -> String {
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
                        .into_iter()
                        .map(Attribute::get_rust)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

impl ToRust for Statement {
    fn to_rust(self, cg: &mut RustCodegen) {
        match self {
            Statement::Block(block) => {
                cg.add_indentedln("{");
                cg.indent += 1;
                block.to_rust(cg);
                cg.indent -= 1;
                cg.add_indentedln("}");
            }

            Statement::VarDecl(VarDeclStmt { decl, init }) => {
                let init = init
                    .map(|e| format!(" = {}", e.get_rust()))
                    .unwrap_or_default();

                cg.add_indentedln(&format!("let {}{}", decl.get_rust(), init));
            }

            Statement::Match(expr, match_items) => {
                cg.add_indentedln(&format!("match {} {{", expr.get_rust()));
                cg.indent += 1;

                for itm in match_items {
                    cg.add_indentedln(&format!(
                        "{} =>",
                        itm.0
                            .into_iter()
                            .map(Pattern::get_rust)
                            .collect::<Vec<_>>()
                            .join(" | ")
                    ));
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
                cg.ensure_brackets(initial.body);

                for else_if_branch in else_if {
                    cg.add_indentedln(&format!("else if {}", else_if_branch.condition.get_rust()));
                    cg.ensure_brackets(else_if_branch.body);
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
                    get_mutable(mutable),
                    pattern.get_rust(),
                    iterator.get_rust()
                ));
                cg.ensure_brackets(body);
            }

            Statement::Return(expr) => {
                let val = expr.map(|e| e.get_rust()).unwrap_or_default();
                cg.add_indentedln(&format!("return {}", val));
            }

            Statement::Break => cg.add_indentedln("break"),
            Statement::Continue => cg.add_indentedln("continue"),
        }
    }
}

impl ToRust for FunctionDecl {
    fn to_rust(self, cg: &mut RustCodegen) {
        let params_str = self
            .params
            .0
            .into_iter()
            .map(VarDecl::get_rust)
            .collect::<Vec<_>>()
            .join(", ");

        cg.add_indentedln(&format!(
            "{}fn {}{}({}) -> {}",
            self.visibility.get_rust(),
            self.name.get_rust(),
            self.generics.get_rust(),
            params_str,
            self.return_type.get_rust()
        ));
        if let Some(body) = self.body {
            cg.add_indentedln("{\n");
            cg.indent += 1;
            body.to_rust(cg);
            cg.indent -= 1;
            cg.add_indentedln("}\n");
        } else {
            cg.add(";");
        }
    }
}

impl ToRust for ImplDecl {
    fn to_rust(self, cg: &mut RustCodegen) {
        if let Some(trait_) = self.trait_ {
            cg.add_indentedln(&format!(
                "impl{} {} for {} {{",
                self.generics.get_rust(),
                trait_.get_rust(),
                self.target.get_rust()
            ));
        } else {
            cg.add_indentedln(&format!(
                "impl{} {} {{",
                self.generics.get_rust(),
                self.target.get_rust()
            ));
        }
        cg.indent += 1;

        for method in self.methods {
            method.to_rust(cg);
        }

        cg.indent -= 1;
        cg.add_indentedln("}");
    }
}

impl GetRust for VarDecl {
    fn get_rust(self) -> String {
        let ty = self
            .type_
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
    fn get_rust(self) -> String {
        self.0
            .into_iter()
            .map(Identifier::get_rust)
            .collect::<Vec<String>>()
            .join("::")
    }
}

impl GetRust for TypePostfix {
    fn get_rust(self) -> String {
        match self {
            TypePostfix::Ref => format!("&"),
            TypePostfix::RefMut => format!("&mut "),
            TypePostfix::RefLifetime(lifetime) => format!("&'{} ", lifetime.get_rust()),
            TypePostfix::RefMutLifetime(lifetime) => format!("&'{} mut ", lifetime.get_rust()),
            TypePostfix::Dyn => format!("dyn "),
        }
    }
}

impl GetRust for Visibility {
    fn get_rust(self) -> String {
        match self {
            Visibility::Public => "pub ".to_string(),
            Visibility::PublicTarget(path) => format!("pub({}) ", path.get_rust()),
            Visibility::Private => "".to_string(),
        }
    }
}

impl GetRust for Identifier {
    fn get_rust(self) -> String {
        self.0.clone()
    }
}

impl GetRust for EnumItem {
    fn get_rust(self) -> String {
        match self {
            Self::Named(id) => id.get_rust(),
            Self::Struct(id, s) => format!(
                "{} {{{}}}",
                id.get_rust(),
                s.into_iter()
                    .map(|field| format!("{}: {}", field.name.get_rust(), field.type_.get_rust()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Tuple(id, t) => format!(
                "{} ({})",
                id.get_rust(),
                t.into_iter()
                    .map(TypeExpr::get_rust)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl GetRust for Pattern {
    fn get_rust(self) -> String {
        match self {
            Self::Id(id) => id.get_rust(),
            Self::Path(path) => path.get_rust(),
            Self::Literal(lit) => lit.get_rust(),
            Self::Struct(path, ids) => format!(
                "{} {{{}}}",
                path.get_rust(),
                ids.into_iter()
                    .map(Identifier::get_rust)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Tuple(ids) => format!(
                "({})",
                ids.into_iter()
                    .map(Identifier::get_rust)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::NamedTuple(path, ids) => {
                format!(
                    "{} ({})",
                    path.get_rust(),
                    ids.into_iter()
                        .map(Identifier::get_rust)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

impl GetRust for GenericsDecl {
    fn get_rust(self) -> String {
        if self.0.len() == 0 {
            String::new()
        } else {
            format!(
                "<{}>",
                self.0
                    .into_iter()
                    .map(|v| v.get_rust())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

impl GetRust for GenericDecl {
    fn get_rust(self) -> String {
        match self {
            GenericDecl::Lifetime(name) => format!("'{}", name.get_rust()),
            GenericDecl::Type(name, requirements) => {
                name.get_rust()
                    + &(if requirements.len() != 0 {
                        format!(
                            ": {}",
                            requirements
                                .into_iter()
                                .map(TypeExpr::get_rust)
                                .collect::<Vec<_>>()
                                .join("+")
                        )
                    } else {
                        String::new()
                    })
            }
        }
    }
}

impl GetRust for Generics {
    fn get_rust(self) -> String {
        if self.0.len() == 0 {
            String::new()
        } else {
            format!(
                "<{}>",
                self.0
                    .into_iter()
                    .map(Generic::get_rust)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

impl GetRust for Generic {
    fn get_rust(self) -> String {
        match self {
            Self::Lifetime(name) => format!("'{}", name.get_rust()),
            Self::Type(ty) => ty.get_rust(),
        }
    }
}

impl GetRust for FieldDecl {
    fn get_rust(self) -> String {
        format!(
            "{}{}: {},",
            self.visibility.get_rust(),
            self.name.get_rust(),
            self.type_.get_rust()
        )
    }
}

pub fn get_static_type_path(path: Path) -> String {
    let rust_path = path.get_rust();

    if rust_path == "void" {
        format!("()")
    } else {
        rust_path
    }
}

pub fn get_type_postfixes(postfixes: Vec<TypePostfix>) -> String {
    postfixes
        .into_iter()
        .map(TypePostfix::get_rust)
        .rev()
        .collect()
}

pub fn get_mutable(mutable: bool) -> String {
    if mutable { "mut " } else { "" }.to_string()
}
