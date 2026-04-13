use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

pub mod ast;
use ast::*;

#[derive(Parser)]
#[grammar = "./src/parser/grammar.pest"]
pub struct MistParser;

// convenience alias for pest errors
pub type ParseError = pest::error::Error<Rule>;

pub fn parse(source: &str) -> Result<Program, ParseError> {
    let pairs = MistParser::parse(Rule::program, source)?;
    let mut statements = vec![];

    for pair in pairs {
        match pair.as_rule() {
            Rule::function_decl => statements.push(TopLevel::Function(parse_function(pair))),
            Rule::struct_decl => statements.push(TopLevel::Struct(parse_struct(pair))),
            Rule::class_decl => statements.push(TopLevel::Class(parse_class(pair))),
            Rule::import_decl => statements.push(TopLevel::Import(parse_import(pair))),
            Rule::EOI => {}
            _ => {}
        }
    }

    Ok(Program { statements })
}

fn span_of(pair: &Pair<Rule>) -> Span {
    let s = pair.as_span();
    Span {
        start: s.start(),
        end: s.end(),
    }
}

fn parse_function(pair: Pair<Rule>) -> Function {
    let span = span_of(&pair);
    let mut inner = pair.into_inner();

    let name = inner.next().unwrap().as_str().to_string();

    let mut params = vec![];
    let mut return_type = None;
    let mut body = vec![];

    for part in inner {
        match part.as_rule() {
            Rule::param_list => params = parse_param_list(part),
            Rule::type_expr => return_type = Some(parse_type_expr(part)),
            Rule::block => body = parse_block(part),
            _ => {}
        }
    }

    Function {
        name,
        params,
        return_type,
        body,
        span,
    }
}

fn parse_param_list(pair: Pair<Rule>) -> Vec<Param> {
    pair.into_inner()
        .map(|p| {
            let span = span_of(&p);
            let mut inner = p.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let type_expr = parse_type_expr(inner.next().unwrap());
            Param {
                name,
                type_expr,
                span,
            }
        })
        .collect()
}

fn parse_struct(pair: Pair<Rule>) -> Struct {
    let span = span_of(&pair);
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let fields = inner.map(|f| parse_struct_field(f)).collect();
    Struct { name, fields, span }
}

fn parse_struct_field(pair: Pair<Rule>) -> StructField {
    let span = span_of(&pair);
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let type_expr = parse_type_expr(inner.next().unwrap());
    StructField {
        name,
        type_expr,
        span,
    }
}

fn parse_class(pair: Pair<Rule>) -> Class {
    let span = span_of(&pair);
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();
    let mut fields = vec![];
    let mut methods = vec![];

    for part in inner {
        match part.as_rule() {
            Rule::struct_field => fields.push(parse_struct_field(part)),
            Rule::function_decl => methods.push(parse_function(part)),
            _ => {}
        }
    }

    Class {
        name,
        fields,
        methods,
        span,
    }
}

fn parse_import(pair: Pair<Rule>) -> Import {
    let span = span_of(&pair);
    let path = pair.into_inner().next().unwrap().as_str().to_string();
    Import { path, span }
}

fn parse_block(pair: Pair<Rule>) -> Vec<Statement> {
    pair.into_inner()
        .filter_map(|p| parse_statement(p))
        .collect()
}

fn parse_statement(pair: Pair<Rule>) -> Option<Statement> {
    match pair.as_rule() {
        Rule::let_stmt => Some(Statement::Let(parse_let(pair))),
        Rule::return_stmt => Some(Statement::Return(parse_return(pair))),
        Rule::if_stmt => Some(Statement::If(parse_if(pair))),
        Rule::for_stmt => Some(Statement::For(parse_for(pair))),
        Rule::expression => Some(Statement::Expression(parse_expression(pair))),
        _ => None,
    }
}

fn parse_let(pair: Pair<Rule>) -> LetStatement {
    let span = span_of(&pair);
    let mut inner = pair.into_inner();
    let name = inner.next().unwrap().as_str().to_string();

    // peek ahead — next is either a type or an expression
    let next = inner.next().unwrap();
    let (type_expr, value) = if next.as_rule() == Rule::type_expr {
        (
            Some(parse_type_expr(next)),
            parse_expression(inner.next().unwrap()),
        )
    } else {
        (None, parse_expression(next))
    };

    LetStatement {
        name,
        type_expr,
        value,
        span,
    }
}

fn parse_return(pair: Pair<Rule>) -> ReturnStatement {
    let span = span_of(&pair);
    let value = pair.into_inner().next().map(|p| parse_expression(p));
    ReturnStatement { value, span }
}

fn parse_if(pair: Pair<Rule>) -> IfStatement {
    let span = span_of(&pair);
    let mut inner = pair.into_inner();
    let condition = parse_expression(inner.next().unwrap());
    let body = parse_block(inner.next().unwrap());
    let else_body = inner.next().map(|p| parse_block(p));
    IfStatement {
        condition,
        body,
        else_body,
        span,
    }
}

fn parse_for(pair: Pair<Rule>) -> ForStatement {
    let span = span_of(&pair);
    let mut inner = pair.into_inner();
    let var = inner.next().unwrap().as_str().to_string();
    let iterator = parse_expression(inner.next().unwrap());
    let body = parse_block(inner.next().unwrap());
    ForStatement {
        var,
        iterator,
        body,
        span,
    }
}

fn parse_expression(pair: Pair<Rule>) -> Expression {
    // let span = span_of(&pair);
    match pair.as_rule() {
        Rule::expression => {
            let mut inner = pair.into_inner();
            let mut expr = parse_primary(inner.next().unwrap());

            // chain field accesses and calls as left-to-right suffixes
            for part in inner {
                let span = span_of(&part);
                match part.as_rule() {
                    Rule::field_access => {
                        let field = part.into_inner().next().unwrap().as_str().to_string();
                        expr = Expression::FieldAccess(Box::new(FieldAccess {
                            object: expr,
                            field,
                            span,
                        }));
                    }
                    Rule::call_suffix => {
                        let args = part.into_inner().map(|p| parse_expression(p)).collect();
                        expr = Expression::Call(Box::new(CallExpr {
                            callee: expr,
                            args,
                            span,
                        }));
                    }
                    _ => {}
                }
            }

            expr
        }
        _ => parse_primary(pair),
    }
}

fn parse_primary(pair: Pair<Rule>) -> Expression {
    let span = span_of(&pair);
    match pair.as_rule() {
        Rule::integer => Expression::Integer(pair.as_str().parse().unwrap(), span),
        Rule::float => Expression::Float(pair.as_str().parse().unwrap(), span),
        Rule::string_lit => {
            Expression::StringLit(pair.into_inner().next().unwrap().as_str().to_string(), span)
        }
        Rule::boolean => Expression::Bool(pair.as_str() == "true", span),
        Rule::identifier => Expression::Identifier(pair.as_str().to_string(), span),
        _ => unreachable!("unexpected primary rule: {:?}", pair.as_rule()),
    }
}

fn parse_type_expr(pair: Pair<Rule>) -> TypeExpr {
    let mut inner = pair.into_inner();
    let base = inner.next().unwrap();

    let base_type = match base.as_rule() {
        Rule::array_type => {
            let inner_type = parse_type_expr(base.into_inner().next().unwrap());
            TypeExpr::Array(Box::new(inner_type))
        }
        Rule::identifier => TypeExpr::Named(base.as_str().to_string()),
        _ => unreachable!(),
    };

    // if a "?" suffix was present, wrap in Optional
    if inner.next().is_some() {
        TypeExpr::Optional(Box::new(base_type))
    } else {
        base_type
    }
}
