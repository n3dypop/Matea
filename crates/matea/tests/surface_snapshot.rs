use matea::{
    core::Runtime,
    prelude::*,
    terminal::{Capabilities, FrameEncoder},
};

struct SnapshotApp;
impl Model for SnapshotApp {
    type Message = ();
    fn update(&mut self, _: Event<()>) -> Effect<()> {
        Effect::None
    }
    fn view(&self, context: ViewContext<'_>) -> View {
        View::surface(
            View::styled("茶 tea", context.theme.accent)
                .with_cursor(Cursor::new(3, 0))
                .render(context.size)
                .unwrap(),
        )
    }
}
#[test]
fn snapshot_scenes_reach_runtime_and_terminal_with_the_same_cells_cursor_and_bytes() {
    let mut program = Runtime::new(SnapshotApp, Size::new(10, 2), Theme::plain()).unwrap();
    let surface = program.frame().unwrap();
    assert_eq!(surface.plain(), "茶 tea\n");
    assert_eq!(surface.cursor(), Some(Cursor::new(3, 0)));
    program.shutdown();
    for color in [
        ColorLevel::None,
        ColorLevel::Ansi16,
        ColorLevel::Ansi256,
        ColorLevel::TrueColor,
    ] {
        for unicode in [false, true] {
            let caps = Capabilities {
                color,
                unicode,
                reduced_motion: true,
            };
            let mut direct = FrameEncoder::default();
            let mut snapshot = FrameEncoder::default();
            let mut a = None;
            let mut b = None;
            for text in ["茶ab\n🙂x", "abc\nx", "茶ab\n🙂x", ""] {
                let scene = View::styled(
                    text,
                    Style::default()
                        .fg(Color::Rgb(19, 41, 227))
                        .underline_style(UnderlineStyle::Curly),
                )
                .with_cursor(Cursor::new(2, 0));
                let frame = scene.render(Size::new(10, 2)).unwrap();
                let via = View::surface(frame.clone())
                    .into_surface(Size::new(10, 2))
                    .unwrap();
                assert_eq!(frame, via);
                let mut x = Vec::new();
                let mut y = Vec::new();
                direct.write(&mut x, &frame, a.as_ref(), caps).unwrap();
                snapshot.write(&mut y, &via, b.as_ref(), caps).unwrap();
                assert_eq!(x, y);
                a = Some(frame);
                b = Some(via);
            }
        }
    }
}
