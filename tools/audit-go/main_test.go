package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
	"time"
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

func directoryLink(t *testing.T, source, destination string) {
	t.Helper()
	if runtime.GOOS == "windows" {
		if output, err := exec.Command("cmd", "/c", "mklink", "/J", destination, source).CombinedOutput(); err != nil {
			t.Fatalf("directory junction: %v: %s", err, output)
		}
	} else if err := os.Symlink(source, destination); err != nil {
		t.Fatal(err)
	}
}

func TestRootMustBeRealDirectoryAndDirectoryLinksAreNotFollowed(t *testing.T) {
	root := fixture(t, "package fixture\nconst LOCAL=1\n")
	outside := fixture(t, "package fixture\nconst OUTSIDE=1\n")
	directoryLink(t, outside, filepath.Join(root, "outside"))
	directoryLink(t, root, filepath.Join(root, "cycle"))
	alias := filepath.Join(t.TempDir(), "alias")
	directoryLink(t, root, alias)
	for _, invalid := range []string{alias, filepath.Join(root, "抹茶.go"), filepath.Join(t.TempDir(), "missing")} {
		var output bytes.Buffer
		if err := audit(invalid, &output); err == nil || output.Len() != 0 {
			t.Fatalf("invalid root accepted/output published: %q, error=%v output=%q", invalid, err, output.String())
		}
	}
	var output bytes.Buffer
	if err := audit(root, &output); err != nil {
		t.Fatal(err)
	}
	var row Entry
	dec := json.NewDecoder(&output)
	if err := dec.Decode(&row); err != nil || row.Symbol != "LOCAL" {
		t.Fatalf("local declaration: %#v %v", row, err)
	}
	if err := dec.Decode(&row); err != io.EOF {
		t.Fatalf("external/cyclic declaration was visited: %#v %v", row, err)
	}
}

func TestFileLinkWithGoSuffixIsNotOpened(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("file symlink creation requires privileges; native directory junctions run separately")
	}
	root := fixture(t, "package fixture\nconst LOCAL=1\n")
	outside := fixture(t, "package fixture\nconst OUTSIDE=1\n")
	if err := os.Symlink(filepath.Join(outside, "抹茶.go"), filepath.Join(root, "escape.go")); err != nil {
		t.Fatal(err)
	}
	var output bytes.Buffer
	if err := audit(root, &output); err != nil {
		t.Fatal(err)
	}
	if strings.Contains(output.String(), "OUTSIDE") || bytes.Count(output.Bytes(), []byte{'\n'}) != 1 {
		t.Fatalf("file link escaped root: %q", output.String())
	}
}

func TestFifoWithGoSuffixIsNotOpened(t *testing.T) {
	if runtime.GOOS == "windows" {
		t.Skip("Unix FIFO fixture")
	}
	root := fixture(t, "package fixture\nconst LOCAL=1\n")
	if output, err := exec.Command("mkfifo", filepath.Join(root, "blocked.go")).CombinedOutput(); err != nil {
		t.Fatalf("mkfifo: %v: %s", err, output)
	}
	var output bytes.Buffer
	done := make(chan error, 1)
	go func() { done <- audit(root, &output) }()
	select {
	case err := <-done:
		if err != nil || !strings.Contains(output.String(), "LOCAL") || bytes.Count(output.Bytes(), []byte{'\n'}) != 1 {
			t.Fatalf("FIFO inventory: error=%v output=%q", err, output.String())
		}
	case <-time.After(2 * time.Second):
		t.Fatal("auditor opened a FIFO instead of skipping the non-regular entry")
	}
}

func TestInvalidRelativeNameFailsBeforeJsonReplacementAndRootCanHaveOsBytes(t *testing.T) {
	if runtime.GOOS != "linux" {
		t.Skip("arbitrary filename bytes require a supporting Linux filesystem")
	}
	root := t.TempDir()
	for _, name := range []string{"\xff.go", "\xfe.go"} {
		if err := os.WriteFile(filepath.Join(root, name), []byte("package fixture\nconst VALUE=1\n"), 0600); err != nil {
			t.Fatal(err)
		}
	}
	var output bytes.Buffer
	if err := audit(root, &output); err == nil || !strings.Contains(err.Error(), "not UTF-8") || output.Len() != 0 {
		t.Fatalf("ambiguous relative identity: error=%v output=%q", err, output.String())
	}
	root = filepath.Join(t.TempDir(), "\xff-root")
	if err := os.Mkdir(root, 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, "lib.go"), []byte("package fixture\nconst LOCAL=1\n"), 0600); err != nil {
		t.Fatal(err)
	}
	output.Reset()
	if err := audit(root, &output); err != nil || !strings.Contains(output.String(), "LOCAL") {
		t.Fatalf("OS-native root: %v %q", err, output.String())
	}
}

func TestSelectedRootIsNotExcludedByItsOwnName(t *testing.T) {
	root := filepath.Join(t.TempDir(), "examples")
	if err := os.Mkdir(root, 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(root, "lib.go"), []byte("package fixture\nconst LOCAL=1\n"), 0600); err != nil {
		t.Fatal(err)
	}
	var output bytes.Buffer
	if err := audit(root, &output); err != nil || !strings.Contains(output.String(), "LOCAL") {
		t.Fatalf("explicitly selected root excluded: %v %q", err, output.String())
	}
}
