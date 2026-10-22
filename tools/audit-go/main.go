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
	"unicode/utf8"
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
	var buffer *bufio.Writer
	var enc *json.Encoder
	err := filepath.WalkDir(root, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if path == root {
			// WalkDir already used Lstat for its root. Reuse that entry instead
			// of making another metadata query; initialize output only after
			// validating the root, before any descendant can emit a row.
			if !d.IsDir() || d.Type()&os.ModeSymlink != 0 {
				return fmt.Errorf("source root must be a real directory")
			}
			buffer = bufio.NewWriterSize(output, 64*1024)
			enc = json.NewEncoder(buffer)
			return nil
		}
		if d.IsDir() {
			switch d.Name() {
			case ".git", "vendor", "node_modules", "examples", "tutorials", "testdata":
				return filepath.SkipDir
			}
			return nil
		}
		// Never open links, pipes or other special entries. The contract is a
		// fixed tree, not protection against concurrent filesystem replacement.
		if !d.Type().IsRegular() || !strings.HasSuffix(path, ".go") {
			return nil
		}
		rel, err := filepath.Rel(root, path)
		if err != nil {
			return err
		}
		rel = filepath.ToSlash(rel)
		if !utf8.ValidString(rel) {
			return fmt.Errorf("source path is not UTF-8: %q", rel)
		}
		return auditSource(rel, path, nil, enc)
	})
	if err != nil {
		return err
	}
	return buffer.Flush()
}

// auditSource parses an existing input (or reads filename when source is nil),
// then serializes every lexical declaration. The caller owns buffering/flush.
func auditSource(rel, filename string, source any, enc *json.Encoder) error {
	var failure error
	fset := token.NewFileSet()
	file, err := parser.ParseFile(fset, filename, source, parser.ParseComments)
	if err != nil {
		return err
	}
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
}
