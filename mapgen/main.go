package main

import (
	"encoding/json"
	"fmt"
	"go/token"
	"go/types"
	"os"

	"golang.org/x/tools/go/packages"
)

type TypeSymbol string

type VarSymbol struct {
	Export  bool       `json:"export"`
	VarType TypeSymbol `json:"var_type"`
	Name    string     `json:"name"`
}

type FunctionSymbol struct {
	Export      bool                 `json:"export"`
	Name        string               `json:"name"`
	Params      map[string]VarSymbol `json:"params"`
	ReturnTypes []TypeSymbol         `json:"return_types"`
}

type StructSymbol struct {
	Export  bool                      `json:"export"`
	Name    string                    `json:"name"`
	Fields  map[string]VarSymbol      `json:"fields"`
	Methods map[string]FunctionSymbol `json:"methods"`
}

type TopLevelSymbolScope struct {
	Structs   map[string]StructSymbol   `json:"structs"`
	Functions map[string]FunctionSymbol `json:"functions"`
}

func main() {
	cfg := &packages.Config{
		Mode: packages.NeedTypes | packages.NeedTypesInfo,
	}

	pkgs, err := packages.Load(cfg, "fmt")
	if err != nil {
		panic(err)
	}

	if packages.PrintErrors(pkgs) > 0 {
		os.Exit(1)
	}

	result := TopLevelSymbolScope{
		Structs:   make(map[string]StructSymbol),
		Functions: make(map[string]FunctionSymbol),
	}

	for _, pkg := range pkgs {
		scope := pkg.Types.Scope()

		for _, name := range scope.Names() {
			if !token.IsExported(name) {
				continue
			}

			obj := scope.Lookup(name)

			switch obj := obj.(type) {

			case *types.Func:
				fn := buildFunction(obj)
				result.Functions[name] = fn

			case *types.TypeName:
				if strct, ok := buildStruct(obj); ok {
					result.Structs[name] = strct
				}
			}
		}
	}

	out, err := json.MarshalIndent(result, "", "  ")
	if err != nil {
		panic(err)
	}

	fmt.Println(string(out))
}

func buildFunction(fn *types.Func) FunctionSymbol {
	sig := fn.Type().(*types.Signature)

	params := make(map[string]VarSymbol)
	for i := 0; i < sig.Params().Len(); i++ {
		p := sig.Params().At(i)

		name := p.Name()
		if name == "" {
			name = fmt.Sprintf("param%d", i)
		}

		params[name] = VarSymbol{
			Export:  true,
			Name:    name,
			VarType: TypeSymbol(p.Type().String()),
		}
	}

	var returnTypes []TypeSymbol
	for i := 0; i < sig.Results().Len(); i++ {
		r := sig.Results().At(i)
		returnTypes = append(returnTypes, TypeSymbol(r.Type().String()))
	}

	return FunctionSymbol{
		Export:      token.IsExported(fn.Name()),
		Name:        fn.Name(),
		Params:      params,
		ReturnTypes: returnTypes,
	}
}

func buildStruct(tn *types.TypeName) (StructSymbol, bool) {
	underlying := tn.Type().Underlying()

	strct, ok := underlying.(*types.Struct)
	if !ok {
		return StructSymbol{}, false
	}

	fields := make(map[string]VarSymbol)

	for i := 0; i < strct.NumFields(); i++ {
		f := strct.Field(i)

		fields[f.Name()] = VarSymbol{
			Export:  token.IsExported(f.Name()),
			Name:    f.Name(),
			VarType: TypeSymbol(f.Type().String()),
		}
	}

	methods := make(map[string]FunctionSymbol)
	methodSet := types.NewMethodSet(tn.Type())

	for i := 0; i < methodSet.Len(); i++ {
		m := methodSet.At(i).Obj().(*types.Func)

		methods[m.Name()] = buildFunction(m)
	}

	return StructSymbol{
		Export:  token.IsExported(tn.Name()),
		Name:    tn.Name(),
		Fields:  fields,
		Methods: methods,
	}, true
}
