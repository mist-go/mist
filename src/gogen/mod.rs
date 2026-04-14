// use crate::parser::ast::{
//     BinOperator, Class, Expression, Function, Statement, Struct, TopLevel, TopLevel, TypeExpr,
// };

// pub fn generate(program: &TopLevel) -> String {
//     let mut out = String::new();

//     out.push_str("package main\n\n");

//     // imports
//     let mut imports = vec![];
//     for item in &program.statements {
//         if let TopLevel::Import(i) = item {
//             imports.push(i.path.clone());
//         }
//     }

//     if !imports.is_empty() {
//         out.push_str("import (\n");
//         for imp in imports {
//             out.push_str(&format!("    {}\n", imp));
//         }
//         out.push_str(")\n\n");
//     }

//     // rest
//     for item in &program.statements {
//         match item {
//             TopLevel::Function(f) => out.push_str(&gen_function(f)),
//             TopLevel::Struct(s) => out.push_str(&gen_struct(s)),
//             TopLevel::Class(c) => out.push_str(&gen_class(c)),
//             TopLevel::Import(_) => {}
//         }
//         out.push('\n');
//     }

//     out
// }

// fn gen_struct(s: &Struct) -> String {
//     let mut out = format!("type {} struct {{\n", s.name);

//     for field in &s.fields {
//         out.push_str(&format!(
//             "    {} {}\n",
//             capitalize(&field.name),
//             gen_type(&field.type_expr)
//         ));
//     }

//     out.push_str("}\n");
//     out
// }

// fn gen_class(c: &Class) -> String {
//     let mut out = String::new();

//     // struct
//     out.push_str(&format!("type {} struct {{\n", c.name));
//     for field in &c.fields {
//         out.push_str(&format!(
//             "    {} {}\n",
//             capitalize(&field.name),
//             gen_type(&field.type_expr)
//         ));
//     }
//     out.push_str("}\n\n");

//     // methods
//     for method in &c.methods {
//         out.push_str(&gen_method(c, method));
//         out.push('\n');
//     }

//     out
// }

// fn gen_function(f: &Function) -> String {
//     let mut out = format!("func {}(", f.name);

//     // params
//     for (i, p) in f.params.iter().enumerate() {
//         if i > 0 {
//             out.push_str(", ");
//         }
//         out.push_str(&format!("{} {}", p.name, gen_type(&p.type_expr)));
//     }

//     out.push(')');

//     // return
//     if let Some(ret) = &f.return_type {
//         out.push_str(&format!(" {}", gen_type(ret)));
//     }

//     out.push_str(" {\n");

//     for stmt in &f.body {
//         out.push_str(&gen_statement(stmt));
//     }

//     out.push_str("}\n");
//     out
// }

// fn gen_method(class: &Class, f: &Function) -> String {
//     let mut out = format!("func (self *{}) {}(", class.name, f.name);

//     for (i, p) in f.params.iter().enumerate() {
//         if i > 0 {
//             out.push_str(", ");
//         }
//         out.push_str(&format!("{} {}", p.name, gen_type(&p.type_expr)));
//     }

//     out.push(')');

//     if let Some(ret) = &f.return_type {
//         out.push_str(&format!(" {}", gen_type(ret)));
//     }

//     out.push_str(" {\n");

//     for stmt in &f.body {
//         out.push_str(&gen_statement(stmt));
//     }

//     out.push_str("}\n");

//     out
// }

// fn gen_statement(stmt: &Statement) -> String {
//     match stmt {
//         Statement::Let(s) => {
//             let mut out = format!("    {} := {}", s.name, gen_expr(&s.value));

//             out.push_str(";\n");
//             out
//         }

//         Statement::Return(r) => match &r.value {
//             Some(v) => format!("    return {};\n", gen_expr(v)),
//             None => "    return;\n".to_string(),
//         },

//         Statement::Expression(e) => {
//             format!("    {};\n", gen_expr(e))
//         }

//         Statement::If(i) => {
//             let mut out = format!("    if {} {{\n", gen_expr(&i.condition));

//             for stmt in &i.body {
//                 out.push_str(&gen_statement(stmt));
//             }

//             out.push_str("    }");

//             if let Some(else_body) = &i.else_body {
//                 out.push_str(" else {\n");
//                 for stmt in else_body {
//                     out.push_str(&gen_statement(stmt));
//                 }
//                 out.push_str("    }");
//             }

//             out.push('\n');
//             out
//         }

//         Statement::For(f) => {
//             let mut out = format!(
//                 "    for _, {} := range {} {{\n",
//                 f.var,
//                 gen_expr(&f.iterator)
//             );

//             for stmt in &f.body {
//                 out.push_str(&gen_statement(stmt));
//             }

//             out.push_str("    }\n");
//             out
//         }
//     }
// }

// fn gen_expr(expr: &Expression) -> String {
//     match expr {
//         Expression::Identifier(name, _) => name.clone(),
//         Expression::Integer(v, _) => v.to_string(),
//         Expression::Float(v, _) => v.to_string(),
//         Expression::StringLit(s, _) => format!("\"{}\"", s),
//         Expression::Bool(b, _) => b.to_string(),

//         Expression::BinaryOp(b) => format!(
//             "{} {} {}",
//             gen_expr(&b.left),
//             op_to_str(&b.op),
//             gen_expr(&b.right)
//         ),

//         Expression::Call(c) => {
//             let args = c.args.iter().map(gen_expr).collect::<Vec<_>>().join(", ");
//             format!("{}({})", gen_expr(&c.callee), args)
//         }

//         Expression::FieldAccess(f) => {
//             format!("{}.{}", gen_expr(&f.object), capitalize(&f.field))
//         }

//         Expression::StructInit(s) => {
//             let mut out = format!("{}{{", s.name);

//             for (i, (name, val)) in s.fields.iter().enumerate() {
//                 if i > 0 {
//                     out.push_str(", ");
//                 }
//                 out.push_str(&format!("{}: {}", capitalize(name), gen_expr(val)));
//             }

//             out.push('}');
//             out
//         }

//         Expression::ArrayLiteral(arr) => {
//             let elems = arr
//                 .elements
//                 .iter()
//                 .map(gen_expr)
//                 .collect::<Vec<_>>()
//                 .join(", ");
//             format!("[]any{{{}}}", elems) // simple version
//         }

//         _ => todo!(),
//     }
// }

// fn gen_type(t: &TypeExpr) -> String {
//     match t {
//         TypeExpr::Named(n) => match n.as_str() {
//             "int" => "int".into(),
//             "float" => "float64".into(),
//             "string" => "string".into(),
//             _ => n.clone(),
//         },

//         TypeExpr::Array(inner) => {
//             format!("[]{}", gen_type(inner))
//         }

//         TypeExpr::Optional(inner) => {
//             format!("*{}", gen_type(inner)) // pointer for optional
//         }
//     }
// }

// fn capitalize(s: &str) -> String {
//     let mut chars = s.chars();
//     match chars.next() {
//         Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
//         None => String::new(),
//     }
// }

// fn op_to_str(op: &BinOperator) -> &'static str {
//     match op {
//         BinOperator::Add => "+",
//         BinOperator::Sub => "-",
//         BinOperator::Mul => "*",
//         BinOperator::Div => "/",
//         BinOperator::Eq => "==",
//         BinOperator::NotEq => "!=",
//         BinOperator::Lt => "<",
//         BinOperator::Gt => ">",
//         BinOperator::LtEq => "<=",
//         BinOperator::GtEq => ">=",
//         BinOperator::And => "&&",
//         BinOperator::Or => "||",
//     }
// }
