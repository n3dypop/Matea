package main

import (
	"encoding/json"
	"io"
	"os"
	"reflect"
	"testing"
)

func captureMeasure(t *testing.T, run func()) []byte {
	t.Helper()
	writer, err := os.CreateTemp(t.TempDir(), "record-*.json")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { writer.Close() })
	previous := os.Stdout
	os.Stdout = writer
	defer func() { os.Stdout = previous }()
	run()
	if _, err := writer.Seek(0, 0); err != nil {
		t.Fatal(err)
	}
	raw, err := io.ReadAll(writer)
	if err != nil {
		t.Fatal(err)
	}
	return raw
}

func TestRecordedIterationsMatchActualWorkAcrossAllPasses(t *testing.T) {
	t.Setenv("MATEA_PROBE_OUTPUT_ONLY", "0")
	t.Setenv("MATEA_PERF_MIN_SAMPLE_NS", "20000000")
	calls := 0
	raw := captureMeasure(t, func() {
		measure("counted", 1, func() uint64 { calls++; return uint64(calls) }, "counted", "independent call counter")
	})
	var record struct {
		Iterations  int       `json:"iterations_per_sample"`
		First       uint64    `json:"first_return_value"`
		Samples     []float64 `json:"samples_ns"`
		Calibration struct {
			Batches []calibrationBatch `json:"batches"`
		} `json:"calibration"`
	}
	if err := json.Unmarshal(raw, &record); err != nil {
		t.Fatal(err)
	}
	// One cold call, 100 warmups, 31 samples, AllocsPerRun's warmup plus
	// ten allocation calls, and ten independent TotalAlloc calls.
	expected := 1 + 100 + 31*record.Iterations + 11 + 10
	for _, batch := range record.Calibration.Batches {
		expected += 3 * batch.Iterations
	}
	if calls != expected || record.First != 1 || len(record.Samples) != 31 {
		t.Fatal(calls, expected, record)
	}
	for _, sample := range record.Samples {
		if sample <= 0 {
			t.Fatal(sample)
		}
	}
}

func TestOutputOnlyCallsOnceBypassesCalibrationAndPropagatesWriterFailure(t *testing.T) {
	t.Setenv("MATEA_PROBE_OUTPUT_ONLY", "1")
	t.Setenv("MATEA_PERF_MIN_SAMPLE_NS", "invalid, deliberately unused")
	calls := 0
	raw := captureMeasure(t, func() { measure("once", 0, func() uint64 { calls++; return 7 }, "text", "output only") })
	var record map[string]any
	if err := json.Unmarshal(raw, &record); err != nil {
		t.Fatal(err)
	}
	if calls != 1 || len(record) != 5 || record["record_kind"] != "probe-output" {
		t.Fatal(calls, record)
	}
	reader, writer, err := os.Pipe()
	if err != nil {
		t.Fatal(err)
	}
	reader.Close()
	writer.Close()
	previous := os.Stdout
	os.Stdout = writer
	defer func() {
		os.Stdout = previous
		if recover() == nil {
			t.Error("closed output was reported successful")
		}
	}()
	measure("closed", 1, func() uint64 { return 0 }, "", "closed writer")
}

func TestCoarseClockCalibrationRetainsZerosAndActualIterations(t *testing.T) {
	durations := []int64{1000, 0, 0, 0, 0, 0, 5, 5, 5, 10, 10, 10}
	var calls []int
	n, history, err := calibratedIterations(3, 10, 100, func(count int) int64 {
		calls = append(calls, count)
		return durations[len(calls)-1]
	})
	if err != nil || n != 24 || !reflect.DeepEqual(calls, []int{3, 3, 3, 6, 6, 6, 12, 12, 12, 24, 24, 24}) {
		t.Fatalf("n=%d history=%v calls=%v err=%v", n, history, calls, err)
	}
	if !reflect.DeepEqual(history, []calibrationBatch{{3, 1000, []int64{0, 0}}, {6, 0, []int64{0, 0}}, {12, 5, []int64{5, 5}}, {24, 10, []int64{10, 10}}}) {
		t.Fatal(history)
	}
}

func TestCalibrationBoundsFailuresAndSaturatedGrowth(t *testing.T) {
	for _, item := range []struct {
		start, limit int
		target       int64
	}{{0, 10, 1}, {10, 9, 1}, {1, 10, 0}} {
		_, _, err := calibratedIterations(item.start, item.target, item.limit, func(int) int64 { t.Fatal("invalid bounds executed workload"); return 0 })
		if err == nil {
			t.Fatal(item)
		}
	}
	var counts []int
	_, history, err := calibratedIterations(3, 10, 10, func(n int) int64 { counts = append(counts, n); return 0 })
	if err == nil || !reflect.DeepEqual(counts, []int{3, 3, 3, 6, 6, 6, 10, 10, 10}) || len(history) != 3 {
		t.Fatal(counts, history, err)
	}
	_, history, err = calibratedIterations(1, 10, 10, func(int) int64 { return -1 })
	if err == nil || len(history) != 0 {
		t.Fatal(history, err)
	}
	n, history, err := calibratedIterations(7, 10, 10, func(int) int64 { return 1000 })
	if err != nil || n != 7 || len(history) != 1 {
		t.Fatal(n, history, err)
	}
}

func TestSampleTargetRequiresExplicitFixedModeAndRejectsInvalidValues(t *testing.T) {
	if n, err := sampleTarget("", false); err != nil || n != 20_000_000 {
		t.Fatal(n, err)
	}
	for _, value := range []string{"0", "1", "1000000", "1000000000"} {
		if _, err := sampleTarget(value, true); err != nil {
			t.Fatal(value, err)
		}
	}
	for _, value := range []string{"", "-1", "1000000001", "nan", "999999999999999999999999"} {
		if _, err := sampleTarget(value, true); err == nil {
			t.Fatal(value)
		}
	}
}
