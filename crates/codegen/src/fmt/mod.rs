pub mod expr;
pub mod statement;
pub mod top_level;

use mist_parser::ast::*;

pub struct Context {
    pub expr_ensure_semicolon: bool,
}

pub struct MistCodegen {
    indent_amount: u8,
    output: String,
    indent: usize,
}

impl MistCodegen {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
            indent_amount: 4,
        }
    }

    fn indent_str(&self) -> String {
        " ".repeat(self.indent_amount as usize).repeat(self.indent)
    }

    pub fn add(&mut self, s: &str) {
        self.output.push_str(s);
    }

    pub fn addln(&mut self, s: &str) {
        self.add(s);
        self.add("\n");
    }

    pub fn add_indented(&mut self, s: &str) {
        let line = format!("{}{}", self.indent_str(), s);
        self.add(&line);
    }

    pub fn add_indentedln(&mut self, s: &str) {
        let line = format!("{}{}\n", self.indent_str(), s);
        self.add(&line);
    }

    pub fn start_bracket(&mut self) {
        self.addln("");
        self.add_indentedln("{");

        self.indent += 1;
    }

    pub fn into_output(self) -> String {
        self.output
    }

    pub fn generate(&mut self, toplevels: Vec<TopLevel>) -> String {
        let mut ctx = Context {
            expr_ensure_semicolon: true,
        };

        for (i, tl) in toplevels.iter().enumerate() {
            if i > 0 {
                self.add("\n");
            }
            tl.gen_mist(&mut ctx, self);
        }

        self.output.clone()
    }

    pub fn ensure_brackets(&mut self, ctx: &mut Context, stmt: &Box<Statement>) {
        match &**stmt {
            Statement::Block(_) => stmt.gen_mist(ctx, self),
            _ => {
                self.add("{");
                self.indent += 1;
                stmt.gen_mist(ctx, self);
                self.indent -= 1;
                self.add_indented("}");
            }
        }
    }

    pub fn ensure_brackets_expr(&mut self, ctx: &mut Context, expr: &Expression) {
        match expr {
            Expression::Statement(stmt) => self.ensure_brackets(ctx, stmt),
            _ => {
                self.add("{");
                self.indent += 1;
                expr.gen_mist(ctx, self);
                self.indent -= 1;
                self.add_indented("}");
            }
        }
    }
}

pub trait GenMist {
    fn gen_mist(&self, ctx: &mut Context, cg: &mut MistCodegen);
}

pub trait GetMist {
    fn get_mist(&self) -> String;
}

impl GetMist for Path {
    fn get_mist(&self) -> String {
        self.0
            .iter()
            .map(Identifier::get_mist)
            .collect::<Vec<String>>()
            .join("::")
    }
}

impl GetMist for Identifier {
    fn get_mist(&self) -> String {
        self.0.clone()
    }
}

impl GetMist for Visibility {
    fn get_mist(&self) -> String {
        match self {
            Visibility::Public => "pub ".to_string(),
            Visibility::PublicTarget(path) => format!("pub({}) ", path.get_mist()),
            Visibility::Private => "".to_string(),
        }
    }
}

impl GetMist for TypeExpr {
    fn get_mist(&self) -> String {
        match self {
            Self::Path(path, generics) => {
                if let Some(generics) = generics {
                    format!("{}{}", path.get_mist(), generics.get_mist())
                } else {
                    path.get_mist()
                }
            }
            Self::Lifetime(name) => format!("'{}", name.get_mist()),
            Self::Tuple(types) => format!(
                "({})",
                types
                    .iter()
                    .map(|t| t.get_mist())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::StaticFn(types, return_type) => {
                if let Some(return_type) = return_type {
                    format!(
                        "fn({}) -> {}",
                        types
                            .iter()
                            .map(|t| t.get_mist())
                            .collect::<Vec<_>>()
                            .join(", "),
                        return_type.get_mist()
                    )
                } else {
                    format!(
                        "fn({})",
                        types
                            .iter()
                            .map(|t| t.get_mist())
                            .collect::<Vec<_>>()
                            .join(", "),
                    )
                }
            }
            Self::UnsafePtr { mutable, ty } => {
                let mutable = if *mutable { "mut " } else { "const " };
                format!("{} {mutable} unsafe&", ty.get_mist())
            }
            Self::Ref {
                lifetime,
                mutable,
                ty,
            } => {
                let base = ty.get_mist();
                if let Some(lifetime) = lifetime {
                    format!(
                        "{} {} '{}&",
                        base,
                        if *mutable { "mut" } else { "" },
                        lifetime.get_mist()
                    )
                } else if *mutable {
                    format!("{} mut&", base)
                } else {
                    format!("{}&", base)
                }
            }
            Self::Dyn(ty) => {
                format!("dyn {}", ty.get_mist())
            }
            Self::Void => "void".to_string(),
            Self::Fn {
                kind,
                return_type,
                params,
            } => {
                format!(
                    "{} {}({})",
                    return_type.get_mist(),
                    kind.get_mist(),
                    params
                        .iter()
                        .map(TypeExpr::get_mist)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

impl GetMist for FnKind {
    fn get_mist(&self) -> String {
        match self {
            Self::Fn => "fn",
            Self::UnsafeFn => "unsafe fn",
            Self::FnClosure => "Fn",
            Self::FnMut => "FnMut",
            Self::FnOnce => "FnOnce",
        }
        .to_string()
    }
}

impl GetMist for ExprPath {
    fn get_mist(&self) -> String {
        self.0
            .iter()
            .map(ExprPathSegment::get_mist)
            .collect::<Vec<_>>()
            .join("::")
    }
}

impl GetMist for ExprPathSegment {
    fn get_mist(&self) -> String {
        format!(
            "{}{}",
            self.ident.get_mist(),
            self.generics
                .as_ref()
                .map(|v| format!("::{}", v.get_mist()))
                .unwrap_or_default()
        )
    }
}

impl GetMist for Generics {
    fn get_mist(&self) -> String {
        if self.0.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                self.0
                    .iter()
                    .map(Generic::get_mist)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

impl GetMist for Generic {
    fn get_mist(&self) -> String {
        match self {
            Self::Lifetime(name) => format!("'{}", name.get_mist()),
            Self::Type(ty) => ty.get_mist(),
        }
    }
}

impl GetMist for GenericsDecl {
    fn get_mist(&self) -> String {
        if self.0.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                self.0
                    .iter()
                    .map(|v| v.get_mist())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

impl GetMist for GenericDecl {
    fn get_mist(&self) -> String {
        match self {
            GenericDecl::Lifetime(name) => format!("'{}", name.get_mist()),
            GenericDecl::Type(name, requirements) => {
                name.get_mist()
                    + &(if !requirements.is_empty() {
                        format!(
                            ": {}",
                            requirements
                                .iter()
                                .map(TypeExpr::get_mist)
                                .collect::<Vec<_>>()
                                .join(" + ")
                        )
                    } else {
                        String::new()
                    })
            }
        }
    }
}

impl<T: GetMist> GenMist for T {
    fn gen_mist(&self, _: &mut Context, cg: &mut MistCodegen) {
        cg.add(&self.get_mist());
    }
}

impl<T: GenMist> GenMist for Spanned<T> {
    fn gen_mist(&self, ctx: &mut Context, cg: &mut MistCodegen) {
        self.item.gen_mist(ctx, cg);
    }
}

impl GenMist for Attribute {
    fn gen_mist(&self, ctx: &mut Context, cg: &mut MistCodegen) {
        match self {
            Self::Path(path) => cg.add(&path.get_mist()),
            Self::NameValue { path, value } => {
                cg.add(&format!("{} = ", path.get_mist()));
                value.gen_mist(ctx, cg);
            }
            Self::List { path, items } => {
                cg.add(&path.get_mist());
                cg.add("(");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        cg.add(", ");
                    }
                    item.gen_mist(ctx, cg);
                }
                cg.add(")");
            }
        }
    }
}

impl GenMist for Pattern {
    fn gen_mist(&self, ctx: &mut Context, cg: &mut MistCodegen) {
        match self {
            Self::Etc => cg.add(".."),
            Self::Literal(lit) => lit.gen_mist(ctx, cg),
            Self::Path(mutable, path) => {
                if *mutable {
                    cg.add("mut ");
                }
                cg.add(&path.get_mist())
            }
            Self::Struct(path, inner) => {
                cg.add(&path.get_mist());
                cg.add(" {");
                for (idx, i) in inner.iter().enumerate() {
                    if idx > 0 {
                        cg.add(", ");
                    }
                    if let Some((name, pat)) = i {
                        cg.add(&name.get_mist());
                        if let Some(pat) = pat {
                            cg.add(": ");
                            pat.gen_mist(ctx, cg);
                        }
                    } else {
                        cg.add("..");
                    }
                }
                cg.add("}");
            }
            Self::NamedTuple(path, inner) => {
                cg.add(&path.get_mist());
                cg.add("(");
                for (i, pat) in inner.iter().enumerate() {
                    if i > 0 {
                        cg.add(", ");
                    }
                    pat.gen_mist(ctx, cg);
                }
                cg.add(")");
            }
            Self::Tuple(inner) => {
                cg.add("(");
                for (i, pat) in inner.iter().enumerate() {
                    if i > 0 {
                        cg.add(", ");
                    }
                    pat.gen_mist(ctx, cg);
                }
                cg.add(")");
            }
        }
    }
}

impl GenMist for Literal {
    fn gen_mist(&self, ctx: &mut Context, cg: &mut MistCodegen) {
        match self {
            Self::Int(n) => cg.add(&n.to_string()),
            Self::Float(n) => cg.add(&format!("{n:?}")),
            Self::Bool(b) => cg.add(&b.to_string()),
            Self::String(s) => cg.add(&format!("\"{s}\"")),
            Self::Tuple(values) => {
                cg.add("(");
                for (i, val) in values.iter().enumerate() {
                    if i > 0 {
                        cg.add(", ");
                    }
                    val.gen_mist(ctx, cg);
                }
                cg.add(")");
            }
        }
    }
}
