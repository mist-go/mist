use std::collections::HashMap;

pub struct TypeSymbol {
    pub export: bool,
    pub name: String,
}

pub struct FunctionSymbol {
    pub export: bool,
    pub name: String,
    pub params: HashMap<String, TypeSymbol>,
}

pub struct StructSymbol {
    pub export: bool,
    pub name: String,
    pub fields: HashMap<String, TypeSymbol>,
    pub methods: HashMap<String, FunctionSymbol>,
}

