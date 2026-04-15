use std::collections::HashMap;

pub struct TypeSymbol {
    export: bool,
    name: String,
}

pub struct FunctionSymbol {
    export: bool,
    name: String,
    params: HashMap<String, TypeSymbol>,
}

pub struct StructSymbol {
    export: bool,
    name: String,
    fields: HashMap<String, TypeSymbol>,
}
