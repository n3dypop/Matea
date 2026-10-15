// Original comparison harness; pinned upstream source remains outside this repository.
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"github.com/charmbracelet/colorprofile"
	"github.com/charmbracelet/harmonica"
	uv "github.com/charmbracelet/ultraviolet"
	"github.com/charmbracelet/x/ansi"
	"math"
	"os"
	"runtime"
	"runtime/debug"
	"strconv"
	"strings"
	"testing"
	"time"
)

var sink uint64

// Mutable package data prevents specializing the update for a constant zero
// velocity. Rust uses black_box inputs for the same purpose.
var motionInputs = [3]float64{2, 0, 1}

func finite(n float64) bool { return !math.IsNaN(n) && !math.IsInf(n, 0) }
func checkedSpring(s harmonica.Spring, p, v, target float64) (float64, float64) {
	if !finite(p) || !finite(v) || !finite(target) {
		panic("nonfinite fixture")
	}
	p, v = s.Update(p, v, target)
	if !finite(p) || !finite(v) {
		panic("nonfinite result")
	}
	return p, v
}

func measure(id string, n int, f func() uint64, output, semantics string) {
	start := time.Now()
	sink = f()
	cold := time.Since(start).Nanoseconds()
	for i := 0; i < 100; i++ {
		sink = f()
	}
	samples := make([]float64, 31)
	for sample := range samples {
		start := time.Now()
		for i := 0; i < n; i++ {
			sink = f()
		}
		samples[sample] = float64(time.Since(start).Nanoseconds()) / float64(n)
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
	json.NewEncoder(os.Stdout).Encode(map[string]any{"case": id, "semantics": semantics, "cold_ns": cold, "iterations_per_sample": n, "samples_ns": samples, "allocations": allocations, "allocated_bytes": float64(after.TotalAlloc-before.TotalAlloc) / 10, "output": output})
}
func plain(s uv.ScreenBuffer) string {
	rows := make([]string, s.Height())
	for y := 0; y < s.Height(); y++ {
		var row strings.Builder
		for x := 0; x < s.Width(); x++ {
			c := s.CellAt(x, y)
			if c == nil {
				row.WriteByte(' ')
			} else if c.Width != 0 {
				row.WriteString(c.Content)
			}
		}
		rows[y] = strings.TrimRight(row.String(), " ")
	}
	return strings.Join(rows, "\n")
}
func main() {
	runtime.GOMAXPROCS(1)
	if len(os.Args) > 1 && os.Args[1] == "--trace" {
		trace()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "--motion-fixtures" {
		motionFixtures()
		return
	}
	n := 300
	if value, err := strconv.Atoi(os.Getenv("MATEA_PERF_ITERATIONS")); err == nil && value > 0 {
		n = value
	}
	for _, item := range []struct{ name, text string }{{"empty", ""}, {"ascii", strings.TrimSuffix(strings.Repeat("Matea tea studio: identical ASCII row\n", 20), "\n")}, {"unicode", strings.TrimSuffix(strings.Repeat("抹茶 e\u0301 👩‍💻 tea\n", 20), "\n")}} {
		view := uv.NewStyledString(item.text)
		sample := uv.NewScreenBuffer(80, 24)
		sample.Method = ansi.GraphemeWidth
		view.Draw(sample, sample.Bounds())
		measure("render/"+item.name+"_80x24", n, func() uint64 {
			s := uv.NewScreenBuffer(80, 24)
			s.Method = ansi.GraphemeWidth
			view.Draw(s, s.Bounds())
			return uint64(s.Width())
		}, plain(sample), "fresh clipped grapheme cell surface, default style, non-CJK ambiguous width")
	}
	text := strings.Repeat("抹茶 e\u0301 👩‍💻 ", 16)
	measure("unicode/width", n*10, func() uint64 { return uint64(ansi.StringWidth(text)) }, strconv.Itoa(ansi.StringWidth(text)), "grapheme display width, no control characters, non-CJK ambiguous width")
	frame := uv.NewScreenBuffer(80, 24)
	frame.Method = ansi.GraphemeWidth
	uv.NewStyledString(strings.Repeat("a", 80)).Draw(frame, frame.Bounds())
	var output bytes.Buffer
	r := uv.NewTerminalRenderer(&output, []string{"TERM=xterm-256color", "NO_COLOR=1"})
	r.SetColorProfile(colorprofile.Ascii)
	r.SetWidthMethod(ansi.GraphemeWidth)
	r.Resize(80, 24)
	r.EnterAltScreen()
	r.Render(frame.RenderBuffer)
	if err := r.Flush(); err != nil {
		panic(err)
	}
	measure("terminal/unchanged_80x24", n, func() uint64 {
		output.Reset()
		r.Render(frame.RenderBuffer)
		if err := r.Flush(); err != nil {
			panic(err)
		}
		return uint64(output.Len())
	}, "", "encode unchanged existing frame to reused memory sink; no terminal I/O")
	flip := true
	measure("terminal/one_cell_80x24", n, func() uint64 {
		output.Reset()
		c := "a"
		if flip {
			c = "b"
		}
		frame.SetCell(0, 0, &uv.Cell{Content: c, Width: 1})
		r.Render(frame.RenderBuffer)
		if err := r.Flush(); err != nil {
			panic(err)
		}
		flip = !flip
		return uint64(output.Len())
	}, "one ASCII cell toggled; independently replay output before claiming equivalence", "toggle one existing ASCII cell, encode to reused memory sink; output encoding may differ")
	for _, damping := range []float64{0.2, 1.0, 2.0} {
		spring := harmonica.NewSpring(0.016, 12, damping)
		p, v := spring.Update(motionInputs[0], motionInputs[1], motionInputs[2])
		expected := fmt.Sprintf("%.12f,%.12f", p, v)
		measure(fmt.Sprintf("motion/spring_update_%g", damping), max(n*10, 30000), func() uint64 {
			p, v := spring.Update(motionInputs[0], motionInputs[1], motionInputs[2])
			return math.Float64bits(p) ^ math.Float64bits(v)
		}, expected, "cached spring update dt=16ms omega=12 state=(2,0) target=1; finite inputs")
		measure(fmt.Sprintf("motion/spring_checked_update_%g", damping), max(n*10, 30000), func() uint64 {
			p, v := checkedSpring(spring, motionInputs[0], motionInputs[1], motionInputs[2])
			return math.Float64bits(p) ^ math.Float64bits(v)
		}, expected, "cached spring update with finite input/result checks dt=16ms omega=12 state=(2,0) target=1")
		measure(fmt.Sprintf("motion/spring_construct_%g", damping), max(n*10, 30000), func() uint64 {
			s := harmonica.NewSpring(0.016, 12, damping)
			p, v := s.Update(motionInputs[0], motionInputs[1], motionInputs[2])
			return math.Float64bits(p) ^ math.Float64bits(v)
		}, expected, "construct spring coefficients and update once dt=16ms omega=12 state=(2,0) target=1; finite inputs")
	}
}
func motionFixtures() {
	for _, d := range []float64{0, 0.2, 0.7, 0.999999, 1, 1.000001, 2, 100, 1e6} {
		for _, w := range []float64{0, 0.1, 4, 12, 100} {
			s := harmonica.NewSpring(0.016, w, d)
			p, v := 2.0, 0.5
			for range 100 {
				p, v = s.Update(p, v, 1)
			}
			fmt.Printf("%.17g,%.17g,%.17g,%.17g\n", d, w, p, v)
		}
	}
}
func trace() {
	frame := uv.NewScreenBuffer(80, 24)
	frame.Method = ansi.GraphemeWidth
	var output bytes.Buffer
	r := uv.NewTerminalRenderer(&output, []string{"TERM=xterm-256color", "NO_COLOR=1"})
	r.SetColorProfile(colorprofile.Ascii)
	r.SetWidthMethod(ansi.GraphemeWidth)
	r.Resize(80, 24)
	r.EnterAltScreen()
	sequences := []string{}
	for _, c := range []string{"a", "b", "a"} {
		output.Reset()
		uv.NewStyledString(c+strings.Repeat("a", 79)).Draw(frame, frame.Bounds())
		r.Render(frame.RenderBuffer)
		if err := r.Flush(); err != nil {
			panic(err)
		}
		sequences = append(sequences, output.String())
	}
	json.NewEncoder(os.Stdout).Encode(map[string]any{"width": 80, "height": 24, "sequences": sequences})
}
