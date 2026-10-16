// Original comparison harness; pinned upstream source remains outside this repository.
package main

import (
	"bytes"
	"charm.land/bubbles/v2/key"
	"charm.land/bubbles/v2/paginator"
	"charm.land/bubbles/v2/textinput"
	tea "charm.land/bubbletea/v2"
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
	"unicode/utf8"
)

var sink uint64

// Mutable package data prevents specializing the update for a constant zero
// velocity. Rust uses black_box inputs for the same purpose.
var motionInputs = [3]float64{2, 0, 1}
var projectilePoint = harmonica.Point{X: 1, Y: 10, Z: -2}
var projectileVelocity = harmonica.Vector{X: 2, Y: -3, Z: 4}
var motionFPS = 60

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
	if len(os.Args) > 1 && os.Args[1] == "--cursor-trace" {
		cursorTrace()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "--motion-fixtures" {
		motionFixtures()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "--projectile-fixtures" {
		projectileFixtures()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "--source-regressions" {
		sourceRegressions()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "--textinput-source-edges" {
		textInputEdges()
		return
	}
	n := 300
	if value, err := strconv.Atoi(os.Getenv("MATEA_PERF_ITERATIONS")); err == nil && value > 0 {
		n = value
	}
	inputBindings(n)
	projectiles(n)
	pagination(n)
	textInput(n)
	cursorEncoding(n)
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
func cursorEncoding(n int) {
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
	r.MoveTo(2, 3)
	r.WriteString(ansi.ShowCursor)
	if err := r.Flush(); err != nil {
		panic(err)
	}
	flip := false
	measure("cursor/visible_move_80x24", n, func() uint64 {
		output.Reset()
		x := 4
		if flip {
			x = 2
		}
		r.Render(frame.RenderBuffer)
		r.MoveTo(x, 3)
		if err := r.Flush(); err != nil {
			panic(err)
		}
		flip = !flip
		return uint64(output.Len())
	}, "visible cursor alternates (2,3),(4,3); independently replay output before claiming equivalence",
		"cursor-only move on identical immutable cells in reused 80x24 frame; visible, default style; reused memory sink; no terminal I/O")
}
func cursorTrace() {
	frame := uv.NewScreenBuffer(80, 24)
	frame.Method = ansi.GraphemeWidth
	uv.NewStyledString(strings.Repeat("a", 80)).Draw(frame, frame.Bounds())
	var output bytes.Buffer
	r := uv.NewTerminalRenderer(&output, []string{"TERM=xterm-256color", "NO_COLOR=1"})
	r.SetColorProfile(colorprofile.Ascii)
	r.SetWidthMethod(ansi.GraphemeWidth)
	r.Resize(80, 24)
	r.EnterAltScreen()
	sequences := []string{}
	for i, x := range []int{2, 4, 2, -1} {
		output.Reset()
		r.Render(frame.RenderBuffer)
		if x >= 0 {
			r.MoveTo(x, 3)
			if i == 0 {
				r.WriteString(ansi.ShowCursor)
			}
		} else {
			r.WriteString(ansi.HideCursor)
		}
		if err := r.Flush(); err != nil {
			panic(err)
		}
		sequences = append(sequences, output.String())
	}
	json.NewEncoder(os.Stdout).Encode(map[string]any{"width": 80, "height": 24, "sequences": sequences})
}
func textInputEdges() {
	altDelete := func(value string) (result string, panicked bool) {
		defer func() { panicked = recover() != nil }()
		input := textinput.New()
		input.SetVirtualCursor(false)
		input.Focus()
		input.SetValue(value)
		input.SetCursor(1)
		input, _ = input.Update(tea.KeyPressMsg{Code: 'd', Mod: tea.ModAlt})
		return input.Value(), false
	}
	inside, _ := altDelete("abc d")
	lastWord, _ := altDelete("ab c")
	_, lastPanics := altDelete("ab")
	downPanics := func() (panicked bool) {
		defer func() { panicked = recover() != nil }()
		input := textinput.New()
		input.SetVirtualCursor(false)
		input.Focus()
		input.ShowSuggestions = true
		input.SetWidth(20)
		input.SetSuggestions([]string{"foo", "bar"})
		input, _ = input.Update(tea.KeyPressMsg{Code: 'f', Text: "f"})
		for range 10 {
			input, _ = input.Update(tea.KeyPressMsg{Code: tea.KeyDown})
			input.View()
			input.CurrentSuggestion()
		}
		return
	}
	emptySelectionPanic := func() (panicked bool) {
		defer func() { panicked = recover() != nil }()
		m := textinput.New()
		m.SetVirtualCursor(false)
		m.Focus()
		m.ShowSuggestions = true
		m, _ = m.Update(tea.KeyPressMsg{Code: tea.KeyUp})
		m.CurrentSuggestion()
		return
	}
	m := textinput.New()
	m.SetVirtualCursor(false)
	m.Focus()
	m.ShowSuggestions = true
	m.CharLimit = 2
	calls := 0
	m.Validate = func(s string) error {
		calls++
		if len(s) != 2 {
			return fmt.Errorf("two bytes")
		}
		return nil
	}
	m.SetSuggestions([]string{"tea"})
	m.SetValue("t")
	staleSuggestion := m.CurrentSuggestion()
	m.SetSuggestions([]string{"tea"})
	before := calls
	m, _ = m.Update(tea.KeyPressMsg{Code: tea.KeyTab})
	placeholder := textinput.New()
	placeholder.Placeholder = "输入消息..."
	placeholder.SetWidth(20)
	zeroWidth := textinput.New()
	zeroWidth.Placeholder = "Something"
	zeroWidth.SetWidth(0)
	native := textinput.New()
	native.Prompt = ""
	native.SetVirtualCursor(false)
	native.Focus()
	native.SetWidth(40)
	native.SetValue("あいう")
	native.CursorEnd()
	scrolled := textinput.New()
	scrolled.Prompt = "> "
	scrolled.SetVirtualCursor(false)
	scrolled.Focus()
	scrolled.SetWidth(20)
	scrolled.SetValue(strings.Repeat("a", 30))
	scrolled.CursorEnd()
	scrolled, _ = scrolled.Update(tea.KeyPressMsg{Code: tea.KeyLeft})
	json.NewEncoder(os.Stdout).Encode(map[string]any{
		"empty_previous_current_suggestion_panics": emptySelectionPanic(),
		"completion_value":                         m.Value(), "completion_limit": m.CharLimit, "validation_calls_before": before, "validation_calls_after": calls,
		"chinese_placeholder_contains_nul":           strings.ContainsRune(placeholder.View(), 0),
		"suggestion_after_set_value_without_refresh": staleSuggestion,
		"alt_delete_inside_word":                     inside, "alt_delete_last_rune_of_word": lastWord, "alt_delete_last_rune_panics": lastPanics,
		"reported_down_case_panics_on_pinned_v2": downPanics(),
		"zero_width_placeholder":                 ansi.Strip(zeroWidth.View()),
		"native_cjk_cursor_x":                    native.Cursor().Position.X, "native_cjk_expected_columns": ansi.StringWidth("あいう"),
		"native_scrolled_left_cursor_x": scrolled.Cursor().Position.X, "native_scrolled_left_expected_columns": 21,
	})
}
func textInput(n int) {
	for _, item := range []struct {
		name, initial string
		position      int
		first, second tea.Msg
	}{
		{"left_right_ascii_64", strings.Repeat("a", 64), 32, tea.KeyPressMsg{Code: tea.KeyLeft}, tea.KeyPressMsg{Code: tea.KeyRight}},
		{"insert_end_ascii_64", strings.Repeat("a", 64), 64, tea.KeyPressMsg{Code: 'x', Text: "x"}, tea.KeyPressMsg{Code: tea.KeyBackspace}},
		{"insert_middle_ascii_64", strings.Repeat("a", 64), 32, tea.KeyPressMsg{Code: 'x', Text: "x"}, tea.KeyPressMsg{Code: tea.KeyBackspace}},
		{"word_roundtrip_ascii_63", strings.Repeat("tea ", 15) + "tea", 63, tea.KeyPressMsg{Code: tea.KeyLeft, Mod: tea.ModAlt}, tea.KeyPressMsg{Code: tea.KeyRight, Mod: tea.ModAlt}},
		{"paste_clear_ascii_64", "", 0, tea.PasteMsg{Content: strings.Repeat("p", 64)}, tea.KeyPressMsg{Code: 'u', Mod: tea.ModCtrl}},
		{"ignored_focus_ascii_64", strings.Repeat("a", 64), 64, tea.BlurMsg{}, tea.FocusMsg{}},
	} {
		m := textinput.New()
		m.SetVirtualCursor(false)
		m.Focus()
		m.SetValue(item.initial)
		m.SetCursor(item.position)
		operation := func() uint64 {
			m, _ = m.Update(item.first)
			m, _ = m.Update(item.second)
			return uint64(len(m.Value()) ^ m.Position())
		}
		operation()
		if m.Value() != item.initial || m.Position() != item.position {
			panic("text fixture")
		}
		semantics := item.name + "; two events; ASCII editing only, no view, virtual cursor, clipboard, validator or suggestions; steady state with capacity reuse"
		measure("textinput/"+item.name, max(n*10, 10000), operation, fmt.Sprintf("%d,%d", len(item.initial), item.position), semantics)
		if m.Value() != item.initial || m.Position() != item.position {
			panic("text fixture drift")
		}
	}
	initial := "抹茶 e\u0301 👩‍💻"
	m := textinput.New()
	m.SetVirtualCursor(false)
	m.Focus()
	m.SetValue(initial)
	m.CursorEnd()
	measure("textinput/word_roundtrip_unicode", max(n*10, 10000), func() uint64 {
		m, _ = m.Update(tea.KeyPressMsg{Code: tea.KeyLeft, Mod: tea.ModAlt})
		m, _ = m.Update(tea.KeyPressMsg{Code: tea.KeyRight, Mod: tea.ModAlt})
		return uint64(len(m.Value()))
	}, fmt.Sprintf("%d,end", len(initial)), "Unicode whitespace word left-right; same value and cursor at end verified outside timing; no view, virtual cursor, clipboard, validator or suggestions")
	if m.Value() != initial || m.Position() != utf8.RuneCountInString(initial) {
		panic("Unicode word fixture")
	}
	m = textinput.New()
	m.SetVirtualCursor(false)
	m.Focus()
	m.ShowSuggestions = true
	m.SetValue("t")
	m.SetSuggestions([]string{"tea", "team", "toast"})
	measure("textinput/suggestions_edit_pair", max(n*10, 10000), func() uint64 {
		m, _ = m.Update(tea.KeyPressMsg{Code: 'e', Text: "e"})
		m, _ = m.Update(tea.KeyPressMsg{Code: tea.KeyBackspace})
		return uint64(len(m.Value()))
	}, "t,3,0", "ASCII t -> te -> t with suggestions tea,team,toast; lowercase prefix filtering; no view, virtual cursor, clipboard or validator")
	if m.Value() != "t" || len(m.MatchedSuggestions()) != 3 || m.CurrentSuggestionIndex() != 0 {
		panic("suggestion fixture")
	}
}
func sourceRegressions() {
	zeroDivisor := func() (panicked bool) {
		defer func() { panicked = recover() != nil }()
		p := paginator.New(paginator.WithPerPage(0))
		p.SetTotalPages(3)
		return
	}
	stale := paginator.New(paginator.WithPerPage(3), paginator.WithTotalPages(10))
	stale.Page = 9
	start, end := stale.GetSliceBounds(4)
	cleared := paginator.New(paginator.WithTotalPages(10))
	cleared.SetTotalPages(0)
	spring := harmonica.NewSpring(0.016, 12, 1e12)
	position, velocity := spring.Update(1e12, 0, 0)
	json.NewEncoder(os.Stdout).Encode(map[string]any{
		"paginator_zero_per_page_panics": zeroDivisor(),
		"stale_slice_bounds":             []int{start, end},
		"stale_items_on_page":            stale.ItemsOnPage(4),
		"pages_after_setting_zero_items": cleared.TotalPages,
		"spring_extreme_overdamping":     map[string]any{"dt": 0.016, "omega": 12, "damping": 1e12, "initial_position": 1e12, "target": 0, "position": position, "velocity": velocity},
	})
}
func checkedProjectile(p *harmonica.Projectile) {
	point := p.Update()
	v := p.Velocity()
	if !finite(point.X) || !finite(point.Y) || !finite(point.Z) || !finite(v.X) || !finite(v.Y) || !finite(v.Z) {
		panic("nonfinite result")
	}
}
func pagination(n int) {
	pages := paginator.New(paginator.WithPerPage(10))
	pages.SetTotalPages(101)
	pages.Page = 10
	start, end := pages.GetSliceBounds(101)
	if start != 100 || end != 101 {
		panic("bounds fixture")
	}
	measure("pagination/range", max(n*10, 300000), func() uint64 { p := pages; a, b := p.GetSliceBounds(101); return uint64(a ^ b) }, "100,101", "collection length=101 per_page=10 page=10; slice bounds only")
	pages.Page = 4
	measure("pagination/navigation", max(n*10, 300000), func() uint64 { p := pages; p.NextPage(); p.PrevPage(); return uint64(p.Page) }, "4", "next then previous from page=4 total_pages=11 per_page=10; state helpers only")
	if pages.View() != "5/11" {
		panic("label fixture")
	}
	measure("pagination/arabic", max(n, 1000), func() uint64 { return uint64(len(pages.View())) }, "5/11", "Arabic label only current=5 total=11; no view/render/I/O")
	for _, count := range []int{32, 256} {
		dots := paginator.New(paginator.WithTotalPages(count))
		dots.Type = paginator.Dots
		dots.Page = 10
		measure(fmt.Sprintf("pagination/dots_%d", count), max(n, 1000), func() uint64 { return uint64(len(dots.View())) }, dots.View(), "dot label only all pages current=10 zero-based; active=• inactive=○; no view/render/I/O")
	}
	measure("pagination/key_dispatch", max(n*10, 30000), func() uint64 {
		pages, _ = pages.Update(tea.KeyPressMsg{Code: tea.KeyRight})
		pages, _ = pages.Update(tea.KeyPressMsg{Code: tea.KeyLeft})
		return uint64(pages.Page)
	}, "4", "next/right then previous/left key dispatch; initial page=4 total_pages=11; no view/render/I/O")
	if pages.Page != 4 {
		panic("navigation fixture")
	}
}
func projectiles(n int) {
	measure("motion/frame_duration", max(n*10, 300000), func() uint64 { return math.Float64bits(harmonica.FPS(motionFPS)) }, fmt.Sprintf("%.12f", harmonica.FPS(60)), "positive fps=60; integer nanosecond period converted to seconds")
	makeProjectile := func() *harmonica.Projectile {
		p, v, a := projectilePoint, projectileVelocity, harmonica.TerminalGravity
		if !finite(p.X) || !finite(p.Y) || !finite(p.Z) || !finite(v.X) || !finite(v.Y) || !finite(v.Z) || !finite(a.X) || !finite(a.Y) || !finite(a.Z) {
			panic("nonfinite fixture")
		}
		return harmonica.NewProjectile(0.016, p, v, a)
	}
	initial := makeProjectile()
	result := *initial
	checkedProjectile(&result)
	p, v := result.Position(), result.Velocity()
	expected := fmt.Sprintf("%.12f,%.12f,%.12f,%.12f,%.12f,%.12f", p.X, p.Y, p.Z, v.X, v.Y, v.Z)
	for _, checked := range []bool{false, true} {
		name, semantics := "motion/projectile_update", "copy initial projectile; raw forward Euler; dt=16ms p=(1,10,-2) v=(2,-3,4) a=(0,9.81,0); read all getters"
		if checked {
			name, semantics = "motion/projectile_checked_update", "copy initial projectile; forward Euler and six finite result checks; dt=16ms p=(1,10,-2) v=(2,-3,4) a=(0,9.81,0); read all getters"
		}
		measure(name, max(n*10, 300000), func() uint64 {
			next := *initial
			if checked {
				checkedProjectile(&next)
			} else {
				next.Update()
			}
			p, v, a := next.Position(), next.Velocity(), next.Acceleration()
			return math.Float64bits(p.X) ^ math.Float64bits(p.Y) ^ math.Float64bits(p.Z) ^ math.Float64bits(v.X) ^ math.Float64bits(v.Y) ^ math.Float64bits(v.Z) ^ math.Float64bits(a.Y)
		}, expected, semantics)
	}
	measure("motion/projectile_construct", max(n*10, 300000), func() uint64 {
		next := makeProjectile()
		checkedProjectile(next)
		p, v := next.Position(), next.Velocity()
		return math.Float64bits(p.Y) ^ math.Float64bits(v.Y)
	}, expected, "construct with nine finite input checks then checked Euler; dt=16ms p=(1,10,-2) v=(2,-3,4) a=(0,9.81,0)")
}
func projectileFixtures() {
	for _, dt := range []int64{0, 1, 16000000, 16666666, 1000000000} {
		for _, steps := range []int{0, 1, 100} {
			for _, a := range []harmonica.Vector{{}, harmonica.Gravity, {X: 0.5, Y: 9.81, Z: -0.2}} {
				p := harmonica.NewProjectile(float64(dt)/1e9, projectilePoint, projectileVelocity, a)
				for range steps {
					p.Update()
				}
				pos, vel := p.Position(), p.Velocity()
				fmt.Printf("%d,%d,%.17g,%.17g,%.17g,%.17g,%.17g,%.17g,%.17g,%.17g,%.17g\n", dt, steps, a.X, a.Y, a.Z, pos.X, pos.Y, pos.Z, vel.X, vel.Y, vel.Z)
			}
		}
	}
}
func inputBindings(n int) {
	for _, item := range []struct {
		name          string
		event         uv.KeyPressEvent
		down, enabled bool
		expected      uint64
	}{
		{"ctrl_x", uv.KeyPressEvent{Code: 'x', Mod: uv.ModCtrl}, true, true, 1},
		{"f12", uv.KeyPressEvent{Code: uv.KeyF12}, true, true, 1},
		{"miss", uv.KeyPressEvent{Code: 'y'}, true, true, 0},
		{"disabled", uv.KeyPressEvent{Code: 'x', Mod: uv.ModCtrl}, true, false, 0},
		{"repeat", uv.KeyPressEvent{Code: 'x', Mod: uv.ModCtrl, IsRepeat: true}, true, true, 1},
		{"release", uv.KeyPressEvent{Code: 'x', Mod: uv.ModCtrl}, false, true, 0},
	} {
		binding := key.NewBinding(key.WithKeys("ctrl+x", "f12"), key.WithHelp("ctrl+x/F12", "inspect"))
		binding.SetEnabled(item.enabled)
		// Bubbles' example widgets call Matches only for KeyPressMsg. The down
		// guard models that event-dispatch contract, including a release case.
		operation := func() uint64 {
			if item.down && key.Matches(item.event, binding) {
				return 1
			}
			return 0
		}
		if operation() != item.expected {
			panic("binding fixture mismatch")
		}
		measure("input/binding_"+item.name, max(n*10, 30000), operation, strconv.FormatUint(item.expected, 10), "two typed/string shortcuts ctrl+x,F12; down-event policy; matching only; no construction or I/O")
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
