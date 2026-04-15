use std::collections::HashMap;

use parser::ast::TopLevel;

use parser::ast::{ParamList, TypeExpr};

pub struct TypeSymbol(pub String);

pub struct VarSymbol {
    pub var_type: TypeSymbol,
    pub name: String,
}

pub struct FunctionSymbol {
    pub export: bool,
    pub name: String,
    pub params: HashMap<String, VarSymbol>,
    pub return_type: Option<TypeSymbol>,
}

pub struct StructSymbol {
    pub export: bool,
    pub name: String,
    pub fields: HashMap<String, VarSymbol>,
    pub methods: HashMap<String, FunctionSymbol>,
}

impl TypeSymbol {
    pub fn from_ast(expr: TypeExpr) -> Self {
        TypeSymbol(match expr {
            TypeExpr::Identifier(i) => i.to_string(),
        })
    }
}

impl FunctionSymbol {
    pub fn from_ast(
        export: bool,
        name: String,
        params: ParamList,
        return_type: Option<TypeExpr>,
    ) -> Self {
        Self {
            export: export,
            name: name.clone(),
            params: params.to_hashmap(|name, v| VarSymbol {
                name: name.clone(),
                var_type: TypeSymbol(match v {
                    TypeExpr::Identifier(i) => i.to_string(),
                }),
            }),
            return_type: return_type.map(TypeSymbol::from_ast),
        }
    }
}

impl StructSymbol {
    pub fn from_ast(export: bool, name: String, fields: ParamList) -> Self {
        Self {
            export,
            name,
            fields: fields.to_hashmap(|name, v| VarSymbol {
                name: name.clone(),
                var_type: TypeSymbol(match v {
                    TypeExpr::Identifier(i) => i.to_string(),
                }),
            }),
            // TODO - parse struct methods
            methods: HashMap::new(),
        }
    }
}

pub struct TopLevelScope {
    pub structs: HashMap<String, StructSymbol>,
    pub functions: HashMap<String, FunctionSymbol>,
}

impl TopLevelScope {
    pub fn from(top_level: &Vec<TopLevel>) -> Self {
        let mut scope = TopLevelScope {
            structs: HashMap::new(),
            functions: HashMap::new(),
        };

        for top in top_level {
            match top {
                TopLevel::Import(_) => unimplemented!(),

                TopLevel::FunctionDecl {
                    export,
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    scope.functions.insert(
                        name.clone(),
                        FunctionSymbol::from_ast(
                            *export,
                            name.clone(),
                            params.clone(),
                            return_type.clone(),
                        ),
                    );
                }

                TopLevel::StructDecl {
                    export,
                    name,
                    fields,
                } => {
                    scope.structs.insert(
                        name.clone(),
                        StructSymbol::from_ast(*export, name.clone(), fields.clone()),
                    );
                }
            }
        }

        scope
    }
}
