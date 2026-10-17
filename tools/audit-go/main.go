// Enumerate public Go declarations from source without compiling upstream dependencies.
package main

import (
	"bufio"
	"bytes"
	"encoding/json"
	"fmt"
	"go/ast"
	"go/format"
	"go/parser"
	"go/token"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"strings"
)

type Entry struct {
	File      string   `json:"file"`
	Line      int      `json:"line"`
	Package   string   `json:"package"`
	Kind      string   `json:"kind"`
	Symbol    string   `json:"symbol"`
	Signature string   `json:"signature,omitempty"`
	Build     []string `json:"build,omitempty"`
}

func main() {
	if len(os.Args) != 2 {
		fmt.Fprintln(os.Stderr, "usage: audit-go SOURCE_ROOT")
		os.Exit(2)
	}
	if err := audit(os.Args[1], os.Stdout); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

func audit(root string, output io.Writer) error {
	buffer := bufio.NewWriterSize(output, 64*1024)
	enc := json.NewEncoder(buffer)
	var failure error
	err := filepath.WalkDir(root, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() {
			switch d.Name() {
			case ".git", "vendor", "node_modules", "examples", "tutorials", "testdata":
				return filepath.SkipDir
			}
			return nil
		}
		if !strings.HasSuffix(path, ".go") {
			return nil
		}
		fset := token.NewFileSet()
		file, err := parser.ParseFile(fset, path, nil, parser.ParseComments)
		if err != nil {
			return err
		}
		rel, err := filepath.Rel(root, path)
		if err != nil {
			return err
		}
		rel = filepath.ToSlash(rel)
		var build []string
		for _, g := range file.Comments {
			for _, c := range g.List {
				if strings.HasPrefix(c.Text, "//go:build ") {
					build = append(build, strings.TrimPrefix(c.Text, "//go:build "))
				}
			}
		}
		signature := func(node any) string {
			var b bytes.Buffer
			if err := format.Node(&b, fset, node); err != nil {
				failure = err
			}
			return b.String()
		}
		emit := func(kind, symbol string, pos token.Pos, node any) {
			if failure != nil {
				return
			}
			e := Entry{File: rel, Line: fset.Position(pos).Line, Package: file.Name.Name, Kind: kind, Symbol: symbol, Build: build}
			if node != nil {
				e.Signature = signature(node)
			}
			if failure != nil {
				return
			}
			if err := enc.Encode(e); err != nil {
				failure = err
			}
		}
		for _, decl := range file.Decls {
			switch decl := decl.(type) {
			case *ast.FuncDecl:
				if strings.HasSuffix(rel, "_test.go") {
					if strings.HasPrefix(decl.Name.Name, "Benchmark") {
						emit("benchmark", decl.Name.Name, decl.Pos(), decl.Type)
					}
					continue
				}
				if !decl.Name.IsExported() {
					continue
				}
				name := decl.Name.Name
				if decl.Recv != nil {
					name = signature(decl.Recv.List[0].Type) + "." + name
				}
				emit("function", name, decl.Pos(), decl.Type)
			case *ast.GenDecl:
				if strings.HasSuffix(rel, "_test.go") {
					continue
				}
				for _, spec := range decl.Specs {
					switch spec := spec.(type) {
					case *ast.TypeSpec:
						if !spec.Name.IsExported() {
							continue
						}
						emit("type", spec.Name.Name, spec.Pos(), nil)
						switch typ := spec.Type.(type) {
						case *ast.StructType:
							for _, field := range typ.Fields.List {
								if len(field.Names) == 0 {
									emit("embedding", spec.Name.Name+"."+signature(field.Type), field.Pos(), field.Type)
								}
								for _, n := range field.Names {
									if n.IsExported() {
										emit("field", spec.Name.Name+"."+n.Name, n.Pos(), field.Type)
									}
								}
							}
						case *ast.InterfaceType:
							for _, field := range typ.Methods.List {
								if len(field.Names) == 0 {
									emit("interface_embedding", spec.Name.Name+"."+signature(field.Type), field.Pos(), field.Type)
								}
								for _, n := range field.Names {
									if n.IsExported() {
										emit("interface_method", spec.Name.Name+"."+n.Name, n.Pos(), field.Type)
									}
								}
							}
						}
					case *ast.ValueSpec:
						for _, n := range spec.Names {
							if n.IsExported() {
								emit(decl.Tok.String(), n.Name, n.Pos(), spec.Type)
							}
						}
					}
				}
			}
		}
		return failure
	})
	if err != nil {
		return err
	}
	return buffer.Flush()
}
