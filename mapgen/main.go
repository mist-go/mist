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
	PackageName string                    `json:"package_name"`
	Structs     map[string]StructSymbol   `json:"structs"`
	Functions   map[string]FunctionSymbol `json:"functions"`
}

func main() {
	cfg := &packages.Config{
		Mode: packages.NeedName |
			packages.NeedTypes |
			packages.NeedTypesInfo |
			packages.NeedModule,
	}

	// you can expand this to multiple packages later
	pkgs, err := packages.Load(cfg, "fmt")
	if err != nil {
		panic(err)
	}

	if packages.PrintErrors(pkgs) > 0 {
		os.Exit(1)
	}

	// 🔑 FINAL RESULT: map[import_path]scope
	result := make(map[string]TopLevelSymbolScope)

	for _, pkg := range pkgs {
		scopeData := TopLevelSymbolScope{
			PackageName: pkg.Name, // e.g. "http"
			Structs:     make(map[string]StructSymbol),
			Functions:   make(map[string]FunctionSymbol),
		}

		scope := pkg.Types.Scope()

		for _, name := range scope.Names() {
			if !token.IsExported(name) {
				continue
			}

			obj := scope.Lookup(name)

			switch obj := obj.(type) {

			case *types.Func:
				scopeData.Functions[name] = buildFunction(obj)

			case *types.TypeName:
				if strct, ok := buildStruct(obj); ok {
					scopeData.Structs[name] = strct
				}
			}
		}

		// 🔑 key = import path (e.g. "net/http")
		result[pkg.PkgPath] = scopeData
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
		Export:      true, // already filtered before calling
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

		if !token.IsExported(f.Name()) {
			continue
		}

		fields[f.Name()] = VarSymbol{
			Export:  true,
			Name:    f.Name(),
			VarType: TypeSymbol(f.Type().String()),
		}
	}

	methods := make(map[string]FunctionSymbol)
	methodSet := types.NewMethodSet(tn.Type())

	for i := 0; i < methodSet.Len(); i++ {
		m := methodSet.At(i).Obj().(*types.Func)

		if !token.IsExported(m.Name()) {
			continue
		}

		methods[m.Name()] = buildFunction(m)
	}

	return StructSymbol{
		Export:  true,
		Name:    tn.Name(),
		Fields:  fields,
		Methods: methods,
	}, true
}
