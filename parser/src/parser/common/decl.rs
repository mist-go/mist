use crate::{
    Rule,
    ast::*,
    ast_expr,
    error::{AstError, GetParseError},
};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for VarDeclStmt {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::var_decl_statement => {
                let mut inner = pair.into_inner();

                let decl = VarDecl::try_from(inner.next().unwrap()).get()?;

                let init = inner.next().map(Expression::try_from).transpose().get()?;

                Ok(VarDeclStmt { decl, init })
            }

            _ => unimplemented!(),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for FieldDeclStmt {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::class_field => {
                let mut inner = pair.into_inner();

                let decl = FieldDecl::try_from(inner.next().unwrap()).get()?;

                let init = inner.next().map(Expression::try_from).transpose().get()?;

                Ok(FieldDeclStmt { decl, init })
            }

            _ => unimplemented!(),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for VarDecl {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::var_decl => {
                let mut inner = pair.into_inner();

                Ok(ast_expr!(inner@VarDecl {
                    type_: & (inner
                        .next()
                        .and_then(|pair| {
                            if pair.as_str().trim() == "var" {
                                None
                            } else {
                                Some(TypeExpr::try_from(pair))
                            }
                        })
                        .transpose()),

                    mutable: ?(Rule::mutable),

                    name: @Pattern
                }))
            }

            _ => unimplemented!("{:?}", pair.as_rule()),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for FieldDecl {
    type Error = AstError<'a, Self>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::field => {
                let mut inner = pair.into_inner();

                Ok(ast_expr!(inner@FieldDecl {
                    visibility: !Visibility,
                    type_: @TypeExpr,
                    name: @Identifier
                }))
            }

            _ => unimplemented!("{:?}", pair.as_rule()),
        }
    }
}
