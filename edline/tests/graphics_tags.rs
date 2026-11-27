use edline::{
    CharAttr16Colors, CharAttr16ColorsDeco, CharAttr256Colors, CharAttr256ColorsDeco,
    CharAttrMonoColor, CharAttrMonoColorDeco, Editor, GraphicsConfig, Output, tag, with_graphics,
};
use embedded_graphics::{mock_display::MockDisplay, pixelcolor::Rgb888};
use embedded_hal_async::delay::DelayNs;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

struct TestDelay;

impl DelayNs for TestDelay {
    async fn delay_ns(&mut self, _ns: u32) {}
}

fn assert_attr<Attr, O: Output<CharAttr = Attr>>(_editor: &Editor<'_, O>)
where
    Attr: edline::CharAttr,
{
}

#[test]
fn graphics_tags_infer_the_character_attribute() {
    macro_rules! check_tag {
        ($tag:expr, $attr:ty) => {{
            let mut line_buf = [0; 16];
            let mut history_buf = [0; 128];
            let mut rollback_buf = [0; 128];
            let editor = with_graphics(
                $tag,
                &mut line_buf,
                &mut history_buf,
                MockDisplay::<Rgb888>::new(),
                &mut rollback_buf,
                GraphicsConfig::default(),
                TestDelay,
                |target| target.set_allow_overdraw(true),
            );
            assert_attr::<$attr, _>(&editor);
        }};
    }

    check_tag!(tag::CharAttrMonoColor, CharAttrMonoColor);
    check_tag!(tag::CharAttrMonoColorDeco, CharAttrMonoColorDeco);
    check_tag!(tag::CharAttr16Colors, CharAttr16Colors);
    check_tag!(tag::CharAttr16ColorsDeco, CharAttr16ColorsDeco);
    check_tag!(tag::CharAttr256Colors, CharAttr256Colors);
    check_tag!(tag::CharAttr256ColorsDeco, CharAttr256ColorsDeco);
    fn assert_tracked_tag(_: impl edline::CharAttrTag<CharAttr = TrackedAttr>) {}
    assert_tracked_tag(PhantomData::<TrackedAttr>);
    assert_tracked_tag(TrackedTag);
}

static ATTR_CREATED: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, PartialEq)]
struct TrackedAttr;

impl edline::CharAttr for TrackedAttr {
    const NBYTES: usize = 0;
    const INVALID: Self = Self;

    fn new() -> Self {
        ATTR_CREATED.fetch_add(1, Ordering::SeqCst);
        Self
    }

    fn has_color(&self) -> bool {
        false
    }
    fn has_deco(&self) -> bool {
        false
    }
    fn is_valid(&self) -> bool {
        true
    }
    fn put_byte(&mut self, _: usize, _: u8) {}
    fn get_bytes(&self) -> &[u8] {
        &[]
    }
    fn get_color_fg(&self) -> edline::Color {
        edline::Color::BLACK
    }
    fn get_color_bg(&self) -> edline::Color {
        edline::Color::WHITE
    }
    fn set_color(self, _: edline::Color, _: edline::Color) -> Self {
        self
    }
    fn set_color_fg(self, _: edline::Color) -> Self {
        self
    }
    fn set_color_bg(self, _: edline::Color) -> Self {
        self
    }
    fn get_deco(&self) -> u8 {
        0
    }
    fn set_deco(self, _: u8) -> Self {
        self
    }
}

struct TrackedTag;

impl edline::CharAttrTag for TrackedTag {
    type CharAttr = TrackedAttr;
}

struct BreakReader;

impl embedded_keycode::ReadKey for BreakReader {
    type Error = core::convert::Infallible;

    async fn read_key(&mut self) -> Result<embedded_keycode::Key, Self::Error> {
        Ok(embedded_keycode::Key::CookedCtrl(embedded_keycode::Ctrl::C))
    }
}

#[test]
fn simple_tasks_use_the_attribute_selected_by_the_tag() {
    let mut buffer = [0; 128];
    let mut line_buf = [0; 128];
    let mut history_buf = [0; 1024];
    let editor = edline::with_console(
        PhantomData::<TrackedAttr>,
        &mut line_buf,
        &mut history_buf,
        &mut buffer[..],
        edline::ConsoleConfig::default().query_text_area_size(false),
    );
    let console = edline::simple_task(editor, BreakReader, true);
    let mut console = std::pin::pin!(console);
    let mut context = Context::from_waker(Waker::noop());
    assert!(matches!(
        console.as_mut().poll(&mut context),
        Poll::Ready(())
    ));
    assert!(ATTR_CREATED.swap(0, Ordering::SeqCst) > 0);

    let mut target = MockDisplay::<Rgb888>::new();
    target.set_allow_overdraw(true);
    let mut line_buf = [0; 128];
    let mut history_buf = [0; 1024];
    let mut rollback_buf = [0; 1024];
    let editor = edline::with_graphics(
        PhantomData::<TrackedAttr>,
        &mut line_buf,
        &mut history_buf,
        target,
        &mut rollback_buf,
        GraphicsConfig::default(),
        TestDelay,
        |_| {},
    );
    let graphics = edline::simple_task(editor, BreakReader, true);
    let mut graphics = std::pin::pin!(graphics);
    assert!(matches!(
        graphics.as_mut().poll(&mut context),
        Poll::Ready(())
    ));
    assert!(ATTR_CREATED.load(Ordering::SeqCst) > 0);
}
