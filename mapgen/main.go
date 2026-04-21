package main

import (
	"fmt"
	"go/token"

	"golang.org/x/tools/go/packages"
)

func main() {
	cfg := &packages.Config{
		Mode: packages.NeedTypes,
	}

	pkgs, err := packages.Load(cfg, "fmt")
	if err != nil {
		panic(err)
	}

	for _, pkg := range pkgs {
		scope := pkg.Types.Scope()

		for _, name := range scope.Names() {
			if !token.IsExported(name) {
				continue
			}

			obj := scope.Lookup(name)
			fmt.Println(name, obj.Type())
		}
	}
}
