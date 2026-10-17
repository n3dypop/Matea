package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func fixture(t *testing.T, source string) string {
	t.Helper()
	root := t.TempDir()
	if err := os.WriteFile(filepath.Join(root, "抹茶.go"), []byte(source), 0600); err != nil {
		t.Fatal(err)
	}
	return root
}

type brokenWriter struct {
	calls   int
	partial bool
}

var broken = errors.New("fixture broken pipe")

func (w *brokenWriter) Write(p []byte) (int, error) {
	w.calls++
	if w.partial {
		return len(p) / 2, nil
	}
	return 0, broken
}
func TestExactOutputAndIgnoredSubdirectories(t *testing.T) {
	root := fixture(t, "package fixture\nconst Answer = 42\nfunc hidden() {}\n")
	for _, name := range []string{"vendor", "examples", "testdata", ".git"} {
		dir := filepath.Join(root, name)
		if err := os.Mkdir(dir, 0700); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(dir, "bad.go"), []byte("invalid Go"), 0600); err != nil {
			t.Fatal(err)
		}
	}
	var output bytes.Buffer
	if err := audit(root, &output); err != nil {
		t.Fatal(err)
	}
	var entries []Entry
	dec := json.NewDecoder(&output)
	for {
		var entry Entry
		err := dec.Decode(&entry)
		if err == io.EOF {
			break
		}
		if err != nil {
			t.Fatal(err)
		}
		entries = append(entries, entry)
	}
	if len(entries) != 1 || entries[0].File != "抹茶.go" || entries[0].Symbol != "Answer" || entries[0].Line != 2 {
		t.Fatalf("unexpected declarations: %#v", entries)
	}
}
func TestParseAndMissingRootErrors(t *testing.T) {
	for _, root := range []string{fixture(t, "package fixture\nfunc Bad( {"), filepath.Join(t.TempDir(), "missing")} {
		err := audit(root, io.Discard)
		if err == nil {
			t.Fatal("invalid source accepted")
		}
	}
}
func TestOutputErrorsAreReturnedIncludingFinalFlush(t *testing.T) {
	for _, count := range []int{1, 10000} {
		for _, partial := range []bool{false, true} {
			var source strings.Builder
			source.WriteString("package fixture\n")
			for i := 0; i < count; i++ {
				source.WriteString("const Public = 1\n")
			}
			writer := &brokenWriter{partial: partial}
			err := audit(fixture(t, source.String()), writer)
			expected := broken
			if partial {
				expected = io.ErrShortWrite
			}
			if !errors.Is(err, expected) || writer.calls != 1 {
				t.Fatalf("count=%d partial=%v calls=%d error=%v", count, partial, writer.calls, err)
			}
		}
	}
}
