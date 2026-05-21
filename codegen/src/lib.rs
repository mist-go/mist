pub mod expr;
pub mod statement;
pub mod top_level;

use mist_parser::ast::*;

pub struct Context {
    pub expr_ensure_semicolon: bool,
}

pub trait GenRust {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen);
}

pub trait GetRust {
    fn get_rust(&self) -> String;
}

#[derive(Default)]
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

    // pub fn generate(&mut self, toplevels: Vec<TopLevel>) -> String {
    //     for tl in toplevels {
    //         tl.gen_rust(self);
    //     }

    //     self.output.clone()
    // }

    // pub fn ensure_brackets(&mut self, stmt: Box<Statement>) {
    //     match *stmt {
    //         Statement::Block(_) => stmt.gen_rust(self),
    //         _ => {
    //             self.add_indentedln("{");
    //             self.indent += 1;
    //             stmt.gen_rust(self);
    //             self.indent -= 1;
    //             self.add_indentedln("}");
    //         }
    //     }
    // }
}

impl<T: GetRust> GetRust for Spanned<T> {
    fn get_rust(&self) -> String {
        format!(
            "/* {}:{} */ {}",
            self.line,
            self.column,
            self.item.get_rust()
        )
    }
}

impl<T: GenRust> GenRust for Spanned<T> {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        cg.add_indentedln(&format!("/* {}:{} */", self.line, self.column));
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

impl GetRust for TypePostfix {
    fn get_rust(&self) -> String {
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
        self.1
            .iter()
            .map(TypePostfix::get_rust)
            .rev()
            .collect::<String>()
            + &self.0.get_rust()
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

impl GetRust for TypeExprKind {
    fn get_rust(&self) -> String {
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

pub fn get_mutable(mutable: bool) -> String {
    if mutable { "mut " } else { "" }.to_string()
}
