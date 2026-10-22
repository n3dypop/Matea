// Internal source-auditor measurements; common harness kept byte-identical to
// tools/perf-upstream/main.go. Ordinary tests only run finite output projections.
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"runtime/debug"
	"strconv"
	"strings"
	"testing"
	"time"
)

var sink uint64

type calibrationBatch struct {
	Iterations      int     `json:"iterations"`
	ElapsedNS       int64   `json:"elapsed_ns"`
	ConfirmationsNS []int64 `json:"confirmations_ns"`
}

// Keep coarse-clock observations as calibration evidence, never timing samples.
// Bound both the arithmetic and work; inability to reach the target is an error.
func calibratedIterations(n int, target int64, limit int, batch func(int) int64) (int, []calibrationBatch, error) {
	if n < 1 || limit < n || target <= 0 {
		return 0, nil, fmt.Errorf("invalid calibration bounds")
	}
	var history []calibrationBatch
	for {
		observation := calibrationBatch{Iterations: n, ConfirmationsNS: make([]int64, 2)}
		ready := true
		for repeat := 0; repeat < 3; repeat++ {
			elapsed := batch(n)
			if elapsed < 0 {
				return 0, history, fmt.Errorf("negative calibration duration")
			}
			if repeat == 0 {
				observation.ElapsedNS = elapsed
			} else {
				observation.ConfirmationsNS[repeat-1] = elapsed
			}
			ready = ready && elapsed >= target
		}
		history = append(history, observation)
		if ready {
			return n, history, nil
		}
		if n == limit {
			return 0, history, fmt.Errorf("sample target not reached at iteration limit")
		}
		if n > limit/2 {
			n = limit
		} else {
			n *= 2
		}
	}
}

func sampleTarget(value string, specified bool) (int64, error) {
	if !specified {
		return 20_000_000, nil
	}
	target, err := strconv.ParseInt(value, 10, 64)
	if err != nil || target < 0 || target > 1_000_000_000 {
		return 0, fmt.Errorf("MATEA_PERF_MIN_SAMPLE_NS must be 0..1000000000; 0 explicitly selects fixed batches")
	}
	return target, nil
}

// Calibration and retained samples must execute exactly the same hot loop.
// Prevent context-dependent inlining/dispatch from changing that cost.
//
//go:noinline
func timedBatch(count int, f func() uint64) int64 {
	start := time.Now()
	for i := 0; i < count; i++ {
		sink = f()
	}
	return time.Since(start).Nanoseconds()
}

func measure(id string, n int, f func() uint64, output, semantics string) {
	if os.Getenv("MATEA_PROBE_OUTPUT_ONLY") == "1" {
		sink = f()
		if err := json.NewEncoder(os.Stdout).Encode(map[string]any{"record_kind": "probe-output", "case": id, "semantics": semantics, "first_return_value": sink, "output": output}); err != nil {
			panic(err)
		}
		return
	}
	if n <= 0 {
		panic("iterations must be positive")
	}
	start := time.Now()
	sink = f()
	firstReturnValue := sink
	cold := time.Since(start).Nanoseconds()
	for i := 0; i < 100; i++ {
		sink = f()
	}
	target, err := sampleTarget(os.LookupEnv("MATEA_PERF_MIN_SAMPLE_NS"))
	if err != nil {
		panic(err)
	}
	starting := n
	const limit = 1 << 26
	var history []calibrationBatch
	if target > 0 {
		n, history, err = calibratedIterations(n, target, limit, func(count int) int64 {
			return timedBatch(count, f)
		})
		if err != nil {
			panic(err)
		}
	}
	samples := make([]float64, 31)
	for sample := range samples {
		elapsed := timedBatch(n, f)
		if elapsed <= 0 {
			panic(fmt.Sprintf("nonpositive timing sample: index=%d iterations=%d target_ns=%d calibration=%v; increase the target/fixed count", sample, n, target, history))
		}
		samples[sample] = float64(elapsed) / float64(n)
	}
	allocations := testing.AllocsPerRun(10, func() { sink = f() })
	old := debug.SetGCPercent(-1)
	var before, after runtime.MemStats
	runtime.ReadMemStats(&before)
	for i := 0; i < 10; i++ {
		sink = f()
	}
	runtime.ReadMemStats(&after)
	debug.SetGCPercent(old)
	record := map[string]any{"case": id, "semantics": semantics, "cold_ns": cold, "first_return_value": firstReturnValue, "iterations_per_sample": n, "samples_ns": samples, "allocations": allocations, "allocated_bytes": float64(after.TotalAlloc-before.TotalAlloc) / 10, "output": output}
	if target > 0 {
		record["calibration"] = map[string]any{"target_ns": target, "start_n": starting, "max_n": limit, "batches": history}
	}
	if err := json.NewEncoder(os.Stdout).Encode(record); err != nil {
		panic(err)
	}
}

type auditPhaseInput struct {
	File string `json:"file"`
	Path string `json:"path"`
	Body []byte `json:"-"`
}
type auditPhaseCase struct {
	Name   string
	Inputs []auditPhaseInput
}

func auditPhaseCases(t *testing.T) []auditPhaseCase {
	t.Helper()
	var cases []auditPhaseCase
	for _, count := range []int{1000, 10000, 100000} {
		for _, structured := range []bool{false, true} {
			var text strings.Builder
			text.WriteString("package fixture\n")
			name := "flat"
			if structured {
				name = "struct"
				text.WriteString("type Group struct {\n")
			}
			for i := 0; i < count; i++ {
				if structured {
					fmt.Fprintf(&text, "F_%d int\n", i)
				} else {
					fmt.Fprintf(&text, "const ITEM_%d = %d\n", i, i)
				}
			}
			if structured {
				text.WriteString("}\n")
			}
			cases = append(cases, auditPhaseCase{Name: fmt.Sprintf("go-audit-parser/%s_%d", name, count), Inputs: []auditPhaseInput{{File: "fixture.go", Path: "fixture.go", Body: []byte(text.String())}}})
		}
	}
	if manifest := os.Getenv("MATEA_AUDIT_GO_MANIFEST"); manifest != "" {
		var external []auditPhaseCase
		data, err := os.ReadFile(manifest)
		if err != nil {
			t.Fatal(err)
		}
		if err = json.Unmarshal(data, &external); err != nil {
			t.Fatal(err)
		}
		for _, c := range external {
			if len(c.Inputs) == 0 {
				t.Fatal("empty source manifest", c.Name)
			}
			for i := range c.Inputs {
				c.Inputs[i].Body, err = os.ReadFile(c.Inputs[i].Path)
				if err != nil {
					t.Fatal(err)
				}
			}
			cases = append(cases, c)
		}
	}
	return cases
}
func auditPhaseBytes(c auditPhaseCase, capacity int) ([]byte, error) {
	var output bytes.Buffer
	output.Grow(capacity)
	encoder := json.NewEncoder(&output)
	for _, source := range c.Inputs {
		if err := auditSource(source.File, source.Path, source.Body, encoder); err != nil {
			return nil, err
		}
	}
	return output.Bytes(), nil
}
func auditPhaseDigest(data []byte) uint64 {
	h := uint64(0xcbf29ce484222325)
	for _, b := range data {
		h = (h ^ uint64(b)) * 0x100000001b3
	}
	return h
}
func TestAuditSourceMeasurement(t *testing.T) {
	mode := os.Getenv("MATEA_GO_AUDIT_PHASE")
	if mode == "" {
		t.Skip("explicit output-only or timing measurement mode")
	}
	if mode != "outputs" && mode != "timing" {
		t.Fatal("invalid MATEA_GO_AUDIT_PHASE")
	}
	previous := runtime.GOMAXPROCS(1)
	defer runtime.GOMAXPROCS(previous)
	if mode == "outputs" {
		t.Setenv("MATEA_PROBE_OUTPUT_ONLY", "1")
	} else {
		t.Setenv("MATEA_PROBE_OUTPUT_ONLY", "0")
	}
	for _, c := range auditPhaseCases(t) {
		expected, err := auditPhaseBytes(c, 0)
		if err != nil {
			t.Fatal(err)
		}
		rows := bytes.Count(expected, []byte{'\n'})
		summary := fmt.Sprintf("rows=%d;bytes=%d;fnv64=%016x", rows, len(expected), auditPhaseDigest(expected))
		measure(c.Name, 1, func() uint64 {
			output, err := auditPhaseBytes(c, len(expected))
			if err != nil {
				panic(err)
			}
			return auditPhaseDigest(output)
		}, summary, "existing fixed source bytes -> Go parser AST -> complete lexical declarations/NDJSON -> FNV64; includes owned output allocation, excludes directory traversal/read/spawn; lexical only, no compiler exports")
	}
}
func TestAuditMeasurementHarnessMatchesCommonCaller(t *testing.T) {
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("missing source location")
	}
	own, err := os.ReadFile(file)
	if err != nil {
		t.Fatal(err)
	}
	shared, err := os.ReadFile(filepath.Join(filepath.Dir(file), "../perf-upstream/main.go"))
	if err != nil {
		t.Fatal(err)
	}
	current := string(own)
	reference := string(shared)
	current = current[strings.Index(current, "type calibrationBatch struct"):strings.Index(current, "type auditPhaseInput struct")]
	reference = reference[strings.Index(reference, "type calibrationBatch struct"):strings.Index(reference, "func plain(")]
	if strings.TrimSpace(current) != strings.TrimSpace(reference) {
		t.Fatal("auditor/shared measurement caller drift")
	}
}
