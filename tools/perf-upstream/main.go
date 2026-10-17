// Original comparison harness; pinned upstream source remains outside this repository.
package main

import (
	"bytes"
	"charm.land/bubbles/v2/help"
	"charm.land/bubbles/v2/key"
	"charm.land/bubbles/v2/paginator"
	"charm.land/bubbles/v2/spinner"
	"charm.land/bubbles/v2/stopwatch"
	"charm.land/bubbles/v2/textinput"
	bubbletimer "charm.land/bubbles/v2/timer"
	tea "charm.land/bubbletea/v2"
	"encoding/json"
	"fmt"
	"github.com/charmbracelet/colorprofile"
	"github.com/charmbracelet/harmonica"
	uv "github.com/charmbracelet/ultraviolet"
	"github.com/charmbracelet/x/ansi"
	"image/color" // Terminal style colors only; no image processing/decoding.
	"math"
	"os"
	"reflect"
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
var bindingSink key.Binding
var clockTextSink string
var clockByteSink []byte

type keyToken string

func (k keyToken) String() string { return string(k) }

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
	if os.Getenv("MATEA_PROBE_OUTPUT_ONLY") == "1" {
		sink = f()
		json.NewEncoder(os.Stdout).Encode(map[string]any{"record_kind": "probe-output", "case": id, "semantics": semantics, "first_return_value": sink, "output": output})
		return
	}
	start := time.Now()
	sink = f()
	firstReturnValue := sink
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
	json.NewEncoder(os.Stdout).Encode(map[string]any{"case": id, "semantics": semantics, "cold_ns": cold, "first_return_value": firstReturnValue, "iterations_per_sample": n, "samples_ns": samples, "allocations": allocations, "allocated_bytes": float64(after.TotalAlloc-before.TotalAlloc) / 10, "output": output})
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
	if len(os.Args) > 1 && os.Args[1] == "--color-trace" {
		coloredFrames(0)
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "--key-source-edges" {
		empty := key.NewBinding(key.WithKeys([]string{}...), key.WithHelp("x", "execute"))
		json.NewEncoder(os.Stdout).Encode(map[string]any{"empty_non_nil_keys_enabled": empty.Enabled(), "empty_non_nil_keys_matches": key.Matches(keyToken("x"), empty), "empty_non_nil_keys_help": empty.Help(), "no_keys_enabled": key.NewBinding().Enabled()})
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
	if len(os.Args) > 1 && os.Args[1] == "--help-source-edges" {
		helpSourceEdges()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "--clock-source-edges" {
		clockSourceEdges()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "--duration-fixtures" {
		durationFixtures()
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "--spinner-fixtures" {
		for _, item := range spinnerPresets() {
			for index, frame := range item.animation.Frames {
				fmt.Printf("%s\t%d\t%d\t%s\n", item.name, item.animation.FPS.Nanoseconds(), index, frame)
			}
		}
		return
	}
	if len(os.Args) > 1 && os.Args[1] == "--spinner-source-edges" {
		spinnerSourceEdges()
		return
	}
	n := 300
	if value, err := strconv.Atoi(os.Getenv("MATEA_PERF_ITERATIONS")); err == nil && value > 0 {
		n = value
	}
	if len(os.Args) > 1 && os.Args[1] == "--clocks" {
		clockCases(n)
		return
	}
	inputBindings(n)
	projectiles(n)
	pagination(n)
	textInput(n)
	cursorEncoding(n)
	coloredFrames(n)
	helpCases(n)
	clockCases(n)
	spinnerCases(n)
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
func startStopwatchForProbe() stopwatch.Model {
	m := stopwatch.New(stopwatch.WithInterval(time.Nanosecond))
	sequence := reflect.ValueOf(m.Start()())
	m, _ = m.Update(sequence.Index(0).Interface().(tea.Cmd)())
	sequence.Index(1).Interface().(tea.Cmd)() // Drain setup timer outside measurement.
	return m
}

type spinnerPreset struct {
	name      string
	animation spinner.Spinner
}

func spinnerPresets() []spinnerPreset {
	return []spinnerPreset{{"line", spinner.Line}, {"dot", spinner.Dot}, {"mini_dot", spinner.MiniDot},
		{"jump", spinner.Jump}, {"pulse", spinner.Pulse}, {"points", spinner.Points},
		{"globe", spinner.Globe}, {"moon", spinner.Moon}, {"monkey", spinner.Monkey},
		{"meter", spinner.Meter}, {"hamburger", spinner.Hamburger}, {"ellipsis", spinner.Ellipsis}}
}
func spinnerSourceEdges() {
	m := spinner.New()
	initial := m.Tick()
	var cmd tea.Cmd
	m, cmd = m.Update(initial)
	first := m.View()
	m, _ = m.Update(initial)
	second := m.View()
	// This real callback has a positive tag, unlike the initial Tick's zero tag.
	positive := cmd()
	before := m.View()
	m, rejected := m.Update(positive)
	positiveRejected := m.View() == before && rejected == nil
	empty := spinner.New(spinner.WithSpinner(spinner.Spinner{Frames: []string{}, FPS: time.Second}))
	empty, _ = empty.Update(empty.Tick())
	m.Spinner.Frames = []string{"only"}
	zero := spinner.New(spinner.WithSpinner(spinner.Spinner{Frames: []string{"a", "b"}, FPS: 0}))
	broadcast := spinner.New()
	broadcast, _ = broadcast.Update(spinner.TickMsg{})
	json.NewEncoder(os.Stdout).Encode(map[string]any{
		"replayed_initial_tick_first": first, "replayed_initial_tick_second": second,
		"old_positive_tag_rejected": positiveRejected, "empty_frames_view": empty.View(),
		"shortened_frames_view": m.View(), "zero_period_accepted_ns": zero.Spinner.FPS.Nanoseconds(),
		"id_zero_broadcast_view": broadcast.View(),
	})
}
func spinnerCases(n int) {
	for _, item := range spinnerPresets() {
		m := spinner.New(spinner.WithSpinner(item.animation))
		measure("spinner/plain_"+item.name, n*10, func() uint64 { return uint64(len(m.View())) }, m.View(),
			"plain spinner frame string from existing immutable animation; storage sharing allowed; no label, style, terminal I/O or scheduling")
	}
	m := spinner.New()
	foreign := spinner.New().Tick()
	measure("spinner/foreign", n*10, func() uint64 {
		var cmd tea.Cmd
		m, cmd = m.Update(foreign)
		if cmd != nil {
			panic("foreign spinner tick accepted")
		}
		return uint64(len(m.View()))
	}, m.View(), "ignore foreign spinner tick; return unchanged plain frame; no new scheduling request")
	m, cmd := m.Update(m.Tick())
	positive := cmd()
	m, _ = m.Update(positive)
	measure("spinner/stale", n*10, func() uint64 {
		var cmd tea.Cmd
		m, cmd = m.Update(positive)
		if cmd != nil {
			panic("stale spinner tick accepted")
		}
		return uint64(len(m.View()))
	}, m.View(), "ignore old positive-generation spinner tick; return unchanged plain frame; no new scheduling request")
}
func clockCases(n int) {
	for _, item := range []struct {
		name  string
		nanos int64
	}{{"zero", 0}, {"nanoseconds", 42}, {"microseconds", 1500}, {"seconds", 1234567890}, {"hours", 3723400000000}, {"maximum_go", 9223372036854775807}} {
		value := time.Duration(item.nanos)
		measure("clocks/format_"+item.name, n, func() uint64 { clockTextSink = value.String(); return uint64(len(clockTextSink)) }, value.String(), "fresh compact nonnegative duration string used by clock View; known duration; no timing, scheduling or terminal I/O")
		clockByteSink = make([]byte, 0, 40)
		measure("clocks/reused_format_"+item.name, n, func() uint64 {
			clockByteSink = append(clockByteSink[:0], value.String()...)
			return uint64(len(clockByteSink))
		}, value.String(), "compact duration appended into cleared caller-owned reusable UTF-8 buffer; known duration; no scheduling or terminal I/O")
	}
	for _, foreign := range []bool{false, true} {
		sw := startStopwatchForProbe()
		other := stopwatch.New()
		var swTick tea.Msg = stopwatch.TickMsg{ID: sw.ID()}
		name := "stopwatch_paused"
		if foreign {
			swTick = stopwatch.TickMsg{ID: other.ID()}
			name = "stopwatch_foreign"
		} else {
			sw, _ = sw.Update(sw.Stop()())
		}
		sw, cmd := sw.Update(swTick)
		output := fmt.Sprintf("value=%dns,running=%t,accepted=%t", sw.Elapsed().Nanoseconds(), sw.Running(), cmd != nil)
		runtime.GC()
		measure("clocks/"+name, n*10, func() uint64 {
			sw, cmd = sw.Update(swTick)
			if cmd != nil {
				panic("ignored tick scheduled work")
			}
			return 0
		}, output, "reject ignored tick, preserve cached clock value/running state, emit no new schedule; prebuilt event; no timer execution")
		timer := bubbletimer.New(10*time.Second, bubbletimer.WithInterval(time.Nanosecond))
		otherTimer := bubbletimer.New(10 * time.Second)
		var timerTick tea.Msg = bubbletimer.TickMsg{ID: timer.ID()}
		name = "countdown_paused"
		if foreign {
			timerTick = bubbletimer.TickMsg{ID: otherTimer.ID()}
			name = "countdown_foreign"
		} else {
			timer, cmd = timer.Update(timer.Stop()())
			if cmd != nil {
				cmd()
			} // Drain pause command's source timer.
		}
		timer, cmd = timer.Update(timerTick)
		output = fmt.Sprintf("value=%dns,running=%t,accepted=%t", timer.Timeout.Nanoseconds(), timer.Running(), cmd != nil)
		runtime.GC()
		measure("clocks/"+name, n*10, func() uint64 {
			timer, cmd = timer.Update(timerTick)
			if cmd != nil {
				panic("ignored tick scheduled work")
			}
			return 0
		}, output, "reject ignored tick, preserve cached clock value/running state, emit no new schedule; prebuilt event; no timer execution")
	}
}
func durationFixtures() {
	for _, n := range []int64{0, 1, 999, 1000, 1001, 999999, 1000000, 1000001, 999999999, 1000000000, 60000000000, 3600000000000, 9223372036854775807} {
		fmt.Printf("%d\t%s\n", n, time.Duration(n).String())
	}
	seed := uint64(237)
	for i := 0; i < 256; i++ {
		seed = seed*6364136223846793005 + 1
		n := int64(seed >> 1)
		fmt.Printf("%d\t%s\n", n, time.Duration(n).String())
	}
}
func clockSourceEdges() {
	result := map[string]any{"stopwatch_default_interval_ns": stopwatch.New().Interval.Nanoseconds()}
	sw := stopwatch.New(stopwatch.WithInterval(time.Millisecond))
	// Reflection only consumes the unexported sequence slice returned by the
	// public Start command. No private fields/tags or modified source are used.
	sequence := reflect.ValueOf(sw.Start()())
	start := sequence.Index(0).Interface().(tea.Cmd)()
	tick := sequence.Index(1).Interface().(tea.Cmd)()
	sw, _ = sw.Update(start)
	sw, _ = sw.Update(tick)
	once := sw.Elapsed()
	sw, _ = sw.Update(tick)
	result["stopwatch_duplicate_initial_tick"] = map[string]any{"once_ns": once.Nanoseconds(), "twice_ns": sw.Elapsed().Nanoseconds()}
	timer := bubbletimer.New(5*time.Millisecond, bubbletimer.WithInterval(time.Millisecond))
	timer, first := timer.Update(timer.Start()())
	timer, second := timer.Update(timer.Start()())
	timer, _ = timer.Update(first())
	timer, _ = timer.Update(second())
	result["timer_two_starts"] = map[string]any{"remaining_ns": timer.Timeout.Nanoseconds(), "scheduled_ticks": 2}
	timer = bubbletimer.New(10*time.Millisecond, bubbletimer.WithInterval(time.Millisecond))
	old := timer.Init()
	timer, _ = timer.Update(timer.Stop()())
	timer, _ = timer.Update(timer.Start()())
	timer, _ = timer.Update(old())
	result["timer_tick_from_before_pause_resume_remaining_ns"] = timer.Timeout.Nanoseconds()
	timer = bubbletimer.New(time.Millisecond, bubbletimer.WithInterval(2*time.Millisecond))
	timer, cmd := timer.Update(bubbletimer.TickMsg{ID: timer.ID()})
	result["timer_overshoot"] = map[string]any{"remaining_ns": timer.Timeout.Nanoseconds(), "view": timer.View(), "followup_command": cmd != nil}
	json.NewEncoder(os.Stdout).Encode(result)
}
func helpData() ([][]key.Binding, []key.Binding) {
	bind := func(k, desc string) key.Binding { return key.NewBinding(key.WithKeys(k), key.WithHelp(k, desc)) }
	groups := [][]key.Binding{{bind("enter", "continue")}, {bind("esc", "back"), bind("?", "help")}, {bind("H", "home"), bind("ctrl+c", "quit"), bind("ctrl+l", "log")}}
	var short []key.Binding
	for _, group := range groups {
		short = append(short, group...)
	}
	return groups, short
}
func helpCases(n int) {
	groups, short := helpData()
	unicode := []key.Binding{key.NewBinding(key.WithKeys("a"), key.WithHelp("茶", "e\u0301を選ぶ")), key.NewBinding(key.WithKeys("b"), key.WithHelp("←/→", "移動")), key.NewBinding(key.WithKeys("q"), key.WithHelp("q", "終了"))}
	for _, fixture := range []struct {
		name  string
		width int
		full  bool
		items []key.Binding
	}{
		{"short_unbounded", 0, false, short}, {"short_40", 40, false, short}, {"full_40", 40, true, short}, {"short_unicode_80", 80, false, unicode},
	} {
		m := help.New()
		m.Styles = help.Styles{}
		m.FullSeparator = " | "
		m.SetWidth(fixture.width)
		render := func() string {
			if fixture.full {
				return m.FullHelpView(groups)
			}
			return m.ShortHelpView(fixture.items)
		}
		measure("help/"+fixture.name, n, func() uint64 { return uint64(len(render())) }, render(), "render fresh plain key help string; existing metadata; no ANSI styling, terminal I/O or binding construction")
	}
}
func helpSourceEdges() {
	groups, _ := helpData()
	m := help.New()
	m.Styles = help.Styles{}
	m.FullSeparator = " | "
	results := map[string]any{}
	for _, width := range []int{1, 2, 3, 16} {
		m.SetWidth(width)
		text := m.ShortHelpView([]key.Binding{key.NewBinding(key.WithKeys("k"), key.WithHelp("k", "go"))})
		results[fmt.Sprintf("short_width_%d", width)] = map[string]any{"text": text, "cell_width": ansi.StringWidth(text)}
	}
	for _, width := range []int{20, 30, 40} {
		m.SetWidth(width)
		results[fmt.Sprintf("full_width_%d", width)] = m.FullHelpView(groups)
	}
	json.NewEncoder(os.Stdout).Encode(results)
}
func coloredFrames(n int) {
	traces := []any{}
	for _, item := range []struct {
		name    string
		profile colorprofile.Profile
		fg, bg  color.RGBA
	}{
		{"ansi16", colorprofile.ANSI, color.RGBA{255, 255, 255, 255}, color.RGBA{0, 0, 0, 255}},
		{"ansi256", colorprofile.ANSI256, color.RGBA{95, 135, 175, 255}, color.RGBA{175, 95, 135, 255}},
		{"truecolor", colorprofile.TrueColor, color.RGBA{95, 135, 175, 255}, color.RGBA{175, 95, 135, 255}},
		{"none", colorprofile.Ascii, color.RGBA{95, 135, 175, 255}, color.RGBA{175, 95, 135, 255}},
	} {
		frame := uv.NewScreenBuffer(80, 24)
		frame.Method = ansi.GraphemeWidth
		for y := 0; y < 20; y++ {
			for x := 0; x < 80; x++ {
				frame.SetCell(x, y, &uv.Cell{Content: "a", Width: 1, Style: uv.Style{Fg: item.fg, Bg: item.bg}})
			}
		}
		var output bytes.Buffer
		r := uv.NewTerminalRenderer(&output, []string{"TERM=xterm-256color"})
		r.SetColorProfile(item.profile)
		r.SetWidthMethod(ansi.GraphemeWidth)
		r.Resize(80, 24)
		r.EnterAltScreen()
		r.WriteString(ansi.HideCursor)
		r.Redraw(frame.RenderBuffer)
		if err := r.Flush(); err != nil {
			panic(err)
		}
		if n == 0 {
			initial := output.String()
			output.Reset()
			r.Resize(80, 24)
			r.WriteString(ansi.ResetStyle)
			r.WriteString(ansi.HideCursor)
			r.Redraw(frame.RenderBuffer)
			if err := r.Flush(); err != nil {
				panic(err)
			}
			traces = append(traces, map[string]any{"profile": item.name, "width": 80, "height": 24, "sequences": []string{initial, output.String()}})
		} else {
			measure("terminal/colored_full_"+item.name+"_80x24", max(n/20, 50), func() uint64 {
				output.Reset()
				r.Resize(80, 24)
				r.WriteString(ansi.ResetStyle)
				r.WriteString(ansi.HideCursor)
				r.Redraw(frame.RenderBuffer)
				if err := r.Flush(); err != nil {
					panic(err)
				}
				return uint64(output.Len())
			}, "20 rows of 80 a cells; palette output replayed independently; lower four rows blank/default",
				"recover unknown cursor/style/visibility with full repaint existing 80x24 frame; 20 ASCII rows, uniform RGB foreground/background; "+item.name+"; hidden cursor; reused memory sink; no terminal I/O")
		}
	}
	if n == 0 {
		json.NewEncoder(os.Stdout).Encode(map[string]any{"traces": traces})
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
	for _, item := range []struct {
		name, value string
		enabled     bool
		expected    uint64
	}{
		{"hit", "F12", true, 1}, {"miss", "other", true, 0}, {"disabled", "F12", false, 0},
	} {
		binding := key.NewBinding(key.WithKeys("ctrl+x", "F12"), key.WithHelp("ctrl+x/F12", "inspect"))
		binding.SetEnabled(item.enabled)
		operation := func() uint64 {
			if key.Matches(keyToken(item.value), binding) {
				return 1
			}
			return 0
		}
		if operation() != item.expected {
			panic("custom token fixture")
		}
		measure("binding/value_"+item.name, max(n*10, 30000), operation, strconv.FormatUint(item.expected, 10), "two custom string tokens ctrl+x,F12; borrowed exact case-sensitive equality; enabled policy; no key formatting or I/O")
	}
	measure("binding/construct", max(n*10, 30000), func() uint64 {
		bindingSink = key.NewBinding(key.WithKeys("ctrl+x", "F12"), key.WithHelp("ctrl+x/F12", "inspect"))
		help := bindingSink.Help()
		return uint64(len(bindingSink.Keys()) + len(help.Key) + len(help.Desc))
	}, "2,ctrl+x/F12,inspect,true", "construct binding with owned two-token vector/slice and short help metadata; retain result through black_box/global sink; borrowed static token text")
	binding := key.NewBinding(key.WithKeys("ctrl+x", "F12"), key.WithHelp("ctrl+x/F12", "inspect"))
	query := strings.Clone("F12")
	if !key.Matches(keyToken(query), binding) {
		panic("owned token fixture")
	}
	measure("binding/value_hit_owned", max(n*10, 30000), func() uint64 {
		if key.Matches(keyToken(query), binding) {
			return 1
		}
		return 0
	}, "1", "two custom string tokens ctrl+x,F12; equal F12 query has distinct owned storage; borrowed equality; no query construction, formatting or I/O")
	measure("binding/metadata", max(n*10, 30000), func() uint64 {
		help := binding.Help()
		enabled := 0
		if binding.Enabled() {
			enabled = 1
		}
		return uint64(len(binding.Keys()) + len(help.Key) + len(help.Desc) + enabled)
	}, "2,ctrl+x/F12,inspect,true", "borrow enabled binding keys, help and enabled status; no mutation, formatting or I/O")
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
