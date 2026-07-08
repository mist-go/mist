pub mod class_decl;
pub mod expr;
pub mod statement;
pub mod top_level;

use std::{collections::HashMap, path::PathBuf};

use mist_parser::{
    ast::*,
    rev_mapper::{Mapping, MistMap, RustMap},
};

pub struct Context {
    pub expr_super: Option<ExprPath>,
}

pub trait GenRust {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen);
}

pub trait GetRust {
    fn get_rust(&self) -> String;
    fn get_rust_ctx(&self, cx: &mut Context) -> String {
        let _ = cx;
        self.get_rust()
    }
}

pub enum Include {
    Glob(Path),
    Use(Visibility, Path),
}

pub struct RustCodegen {
    output: String,
    indent: usize,
    pub crates: HashMap<Identifier, Vec<Include>>,
    pub mapping: Mapping,
    position: RustMap,
}

impl RustCodegen {
    pub fn new(mist_path: PathBuf) -> Self {
        Self {
            output: String::new(),
            indent: 0,
            crates: HashMap::new(),
            mapping: Mapping::new(mist_path),
            position: RustMap(1, 0),
        }
    }

    fn indent_str(&self) -> String {
        "    ".repeat(self.indent)
    }

    pub fn add(&mut self, s: &str) {
        let newline_count = bytecount::count(s.as_bytes(), b'\n');

        if newline_count == 0 {
            self.position.1 += s.len();
        } else {
            self.position.0 += newline_count;

            let last_newline = s.rfind('\n').unwrap();
            self.position.1 = s.len() - last_newline - 1;
        }

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

    pub fn generate(&mut self, toplevels: Vec<TopLevel>) -> String {
        let mut ctx = Context { expr_super: None };

        for tl in toplevels {
            tl.gen_rust(&mut ctx, self);
        }

        let mut crates = HashMap::new();
        std::mem::swap(&mut self.crates, &mut crates);

        for (c, items) in crates {
            self.add("mod ");
            c.gen_rust(&mut ctx, self);
            self.addln(" {");
            self.indent += 1;

            self.add_indented("extern crate ");
            c.gen_rust(&mut ctx, self);
            self.addln(";");

            for item in items {
                self.add_indented("pub use ");
                match item {
                    Include::Glob(item) => {
                        item.gen_rust(&mut ctx, self);
                        self.add("::*");
                    }
                    Include::Use(vis, item) => {
                        vis.gen_rust(&mut ctx, self);
                        item.gen_rust(&mut ctx, self);
                    }
                }
                self.addln(";");
            }

            self.indent -= 1;
            self.addln("}");
        }

        self.output.clone()
    }

    pub fn ensure_brackets(&mut self, ctx: &mut Context, stmt: &Box<Statement>) {
        match &**stmt {
            Statement::Block(_) => stmt.gen_rust(ctx, self),
            _ => {
                self.add("{");
                self.indent += 1;
                stmt.gen_rust(ctx, self);
                self.indent -= 1;
                self.add("}");
            }
        }
    }

    pub fn ensure_brackets_expr(&mut self, ctx: &mut Context, expr: &Expression) {
        match expr {
            Expression::Statement(stmt) => self.ensure_brackets(ctx, stmt),
            _ => {
                self.add("{");
                self.indent += 1;
                expr.gen_rust(ctx, self);
                self.indent -= 1;
                self.add("}");
            }
        }
    }
}

impl GenRust for Attribute {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        match self {
            Self::Path(path) => cg.add(&path.get_rust()),
            Self::NameValue { path, value } => {
                cg.add(&format!("{} = ", path.get_rust()));
                value.gen_rust(ctx, cg);
            }
            Self::List { path, items } => {
                cg.add(&path.get_rust());
                cg.add("(");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        cg.add(", ");
                    }

                    item.gen_rust(ctx, cg);
                }
                cg.add(")");
            }
        }
    }
}

pub trait GenSpanTranslation {
    fn gen_span(&self, _cg: &mut RustCodegen);
}

impl<T> GenSpanTranslation for Spanned<T> {
    fn gen_span(&self, cg: &mut RustCodegen) {
        cg.mapping
            .map
            .insert((cg.position, MistMap(self.line, self.column)));
    }
}

impl<T: GetRust> GenRust for T {
    fn gen_rust(&self, _: &mut Context, cg: &mut RustCodegen) {
        cg.add(&self.get_rust());
    }
}

impl<T: GenRust> GenRust for Spanned<T> {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        self.gen_span(cg);
        self.item.gen_rust(ctx, cg);
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

impl GetRust for Visibility {
    fn get_rust(&self) -> String {
        match self {
            Visibility::Public => "pub ".to_string(),
            Visibility::PublicTarget(path) => format!("pub({}) ", path.get_rust()),
            Visibility::Private => "".to_string(),
        }
    }
}

impl GetRust for Identifier {
    fn get_rust(&self) -> String {
        self.0.clone()
    }
}

impl GetRust for TypeExpr {
    fn get_rust(&self) -> String {
        match self {
            Self::Path(path, generics) => {
                if let Some(generics) = generics {
                    format!("{}{}", path.get_rust(), generics.get_rust())
                } else {
                    path.get_rust()
                }
            }
            Self::Lifetime(name) => format!("'{}", name.get_rust()),
            Self::Tuple(types) => format!(
                "({})",
                types
                    .into_iter()
                    .map(|t| t.get_rust())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::StaticFn(types, return_type) => {
                if let Some(return_type) = return_type {
                    format!(
                        "fn({}) -> {}",
                        types
                            .into_iter()
                            .map(|t| t.get_rust())
                            .collect::<Vec<_>>()
                            .join(", "),
                        return_type.get_rust()
                    )
                } else {
                    format!(
                        "fn({})",
                        types
                            .into_iter()
                            .map(|t| t.get_rust())
                            .collect::<Vec<_>>()
                            .join(", "),
                    )
                }
            }

            Self::UnsafePtr { mutable, ty } => {
                let mutable = if *mutable { "mut " } else { "const " };
                format!("*{mutable}{}", ty.get_rust())
            }

            Self::Ref {
                lifetime,
                mutable,
                ty,
            } => {
                if let Some(lifetime) = lifetime {
                    format!(
                        "&'{} {}{}",
                        lifetime.get_rust(),
                        if *mutable { "mut " } else { "" },
                        ty.get_rust()
                    )
                } else {
                    format!("&{}{}", if *mutable { "mut " } else { "" }, ty.get_rust())
                }
            }

            Self::Dyn(ty) => {
                format!("dyn {}", ty.get_rust())
            }
            Self::Void => "()".to_string(),
            Self::Fn {
                kind,
                return_type,
                params,
            } => {
                format!(
                    "{}({}) -> {}",
                    kind.get_rust(),
                    params
                        .iter()
                        .map(TypeExpr::get_rust)
                        .collect::<Vec<_>>()
                        .join(", "),
                    return_type.get_rust(),
                )
            }
            Self::Array(ty, count) => {
                if let Some(count) = count {
                    format!("[{}; {count}]", ty.get_rust())
                } else {
                    format!("[{}]", ty.get_rust())
                }
            }
        }
    }
}

impl GetRust for FnKind {
    fn get_rust(&self) -> String {
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

impl GenRust for Pattern {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        match self {
            Self::Etc => cg.add(".."),
            Self::Literal(lit) => lit.gen_rust(ctx, cg),
            Self::Path(mutable, path) => {
                if *mutable {
                    cg.add("mut ");
                };

                cg.add(&path.get_rust())
            }
            Self::Struct(path, inner) => {
                cg.add(&path.get_rust());
                cg.add(" {");

                for (idx, i) in inner.iter().enumerate() {
                    if idx > 0 {
                        cg.add(", ");
                    }

                    if let Some((name, pat)) = i {
                        cg.add(&name.get_rust());
                        if let Some(pat) = pat {
                            cg.add(": ");
                            pat.gen_rust(ctx, cg);
                        }
                    } else {
                        cg.add("..");
                    }
                }

                cg.add("}");
            }

            Self::NamedTuple(path, inner) => {
                cg.add(&path.get_rust());
                cg.add(" (");
                for pat in inner {
                    pat.gen_rust(ctx, cg);
                    cg.add(",");
                }
                cg.add(")");
            }

            Self::Tuple(inner) => {
                cg.add("(");
                for pat in inner {
                    pat.gen_rust(ctx, cg);
                    cg.add(",");
                }
                cg.add(")");
            }
        }
    }
}
