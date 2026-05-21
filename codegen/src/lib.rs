pub mod expr;
pub mod statement;
pub mod top_level;

use mist_parser::ast::*;

pub trait GenRust {
    fn get_rust(&self, cg: &mut RustCodegen);
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
