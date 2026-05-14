use crate::{Rule, ast::*, error::ParseError, parser::listen_rule};

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for VarDeclStmt {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::var_decl_statement => {
                let mut inner = pair.into_inner();

                let decl = VarDecl::try_from(inner.next().unwrap())?;

                let init = inner.next().map(Expression::try_from).transpose()?;

                Ok(VarDeclStmt { decl, init })
            }

            _ => unimplemented!(),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for FieldDeclStmt {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::class_field => {
                let mut inner = pair.into_inner();

                let decl = FieldDecl::try_from(inner.next().unwrap())?;

                let init = inner.next().map(Expression::try_from).transpose()?;

                Ok(FieldDeclStmt { decl, init })
            }

            _ => unimplemented!(),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for VarDecl {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::var_decl => {
                let mut inner = pair.into_inner();

                let type_ = inner
                    .next()
                    .and_then(|pair| {
                        if pair.as_str().trim() == "var" {
                            None
                        } else {
                            Some(TypeExpr::try_from(pair))
                        }
                    })
                    .transpose()?;

                let mutable = listen_rule(&mut inner, Rule::mutable);

                let name = Pattern::try_from(inner.next().unwrap())?;

                Ok(VarDecl {
                    mutable,
                    name,
                    type_,
                })
            }

            _ => unimplemented!("{:?}", pair.as_rule()),
        }
    }
}

impl<'a> TryFrom<pest::iterators::Pair<'a, Rule>> for FieldDecl {
    type Error = ParseError<'a>;

    fn try_from(pair: pest::iterators::Pair<'a, Rule>) -> Result<Self, Self::Error> {
        match pair.as_rule() {
            Rule::field => {
                let mut inner = pair.into_inner();

                let visibility = Visibility::try_from(&mut inner)?;
                let type_ = TypeExpr::try_from(inner.next().unwrap())?;
                let name = Identifier::try_from(inner.next().unwrap())?;

                Ok(FieldDecl {
                    visibility,
                    type_,
                    name,
                })
            }

            _ => unimplemented!("{:?}", pair.as_rule()),
        }
    }
}
