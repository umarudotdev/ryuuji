//! Modal overlay widget.
//!
//! Shows content centered over a semi-transparent backdrop.
//! Clicking the backdrop or pressing Escape triggers the `on_blur` message.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer::{self, Quad};
use iced::advanced::widget::{self, Tree, Widget};
use iced::advanced::{Clipboard, Renderer as _, Shell};
use iced::mouse;
use iced::{
    Border, Color, Element, Event, Length, Point, Rectangle, Size, Theme, Vector,
};

/// Modal backdrop color — deep semi-transparent black.
/// Works well for both light and dark themes.
const MODAL_BACKDROP: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.65,
};

// ── Public API ──────────────────────────────────────────────────────

/// Wrap `base` with a modal overlay showing `content` over a backdrop.
///
/// When the user clicks the backdrop or presses Escape, `on_blur` is
/// published.
pub fn modal<'a, Message: Clone + 'a>(
    base: impl Into<Element<'a, Message>>,
    content: impl Into<Element<'a, Message>>,
    on_blur: Message,
) -> Element<'a, Message> {
    Modal {
        base: base.into(),
        modal_content: Some(content.into()),
        on_blur,
    }
    .into()
}

// ── Internal types ──────────────────────────────────────────────────

struct Modal<'a, Message> {
    base: Element<'a, Message>,
    modal_content: Option<Element<'a, Message>>,
    on_blur: Message,
}

// ── Widget impl ─────────────────────────────────────────────────────

impl<'a, Message: Clone + 'a> Widget<Message, Theme, iced::Renderer>
    for Modal<'a, Message>
{
    fn children(&self) -> Vec<Tree> {
        let content = self.modal_content.as_ref().unwrap();
        vec![Tree::new(&self.base), Tree::new(content)]
    }

    fn diff(&self, tree: &mut Tree) {
        let content = self.modal_content.as_ref().unwrap();
        tree.diff_children(&[&self.base, content]);
    }

    fn size(&self) -> Size<Length> {
        self.base.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.base
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.base.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        _event: &Event,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn Clipboard,
        _shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        // Don't forward events to base — the modal blocks interaction.
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::None
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        _layout: Layout<'b>,
        _renderer: &iced::Renderer,
        viewport: &Rectangle,
        _translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        // Take the content out so the overlay can own it with lifetime 'b.
        let content = self.modal_content.take()?;
        Some(overlay::Element::new(Box::new(ModalOverlay {
            content,
            tree: &mut tree.children[1],
            on_blur: self.on_blur.clone(),
            viewport: *viewport,
        })))
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.base.as_widget_mut().operate(
            &mut tree.children[0],
            layout,
            renderer,
            operation,
        );
    }
}

impl<'a, Message: Clone + 'a> From<Modal<'a, Message>>
    for Element<'a, Message>
{
    fn from(m: Modal<'a, Message>) -> Self {
        Element::new(m)
    }
}

// ── Overlay impl ────────────────────────────────────────────────────

struct ModalOverlay<'a, Message> {
    content: Element<'a, Message>,
    tree: &'a mut Tree,
    on_blur: Message,
    viewport: Rectangle,
}

impl<'a, Message: Clone + 'a>
    overlay::Overlay<Message, Theme, iced::Renderer>
    for ModalOverlay<'a, Message>
{
    fn layout(
        &mut self,
        renderer: &iced::Renderer,
        bounds: Size,
    ) -> layout::Node {
        let limits = layout::Limits::new(Size::ZERO, bounds);
        let node = self
            .content
            .as_widget_mut()
            .layout(self.tree, renderer, &limits);

        let content_size = node.size();

        // Center in viewport.
        let x = (bounds.width - content_size.width) / 2.0;
        let y = (bounds.height - content_size.height) / 2.0;

        node.move_to(Point::new(x.max(0.0), y.max(0.0)))
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        // Draw semi-transparent backdrop.
        renderer.fill_quad(
            Quad {
                bounds: self.viewport,
                border: Border::default(),
                ..Quad::default()
            },
            MODAL_BACKDROP,
        );

        // Draw modal content.
        self.content.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &self.viewport,
        );
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        // Forward events to modal content.
        self.content.as_widget_mut().update(
            self.tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &self.viewport,
        );

        if shell.is_event_captured() {
            return;
        }

        match event {
            // Click outside -> dismiss.
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if cursor.position_over(layout.bounds()).is_none() {
                    shell.publish(self.on_blur.clone());
                    shell.capture_event();
                }
            }
            // Escape -> dismiss.
            Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key:
                    iced::keyboard::Key::Named(
                        iced::keyboard::key::Named::Escape,
                    ),
                ..
            }) => {
                shell.publish(self.on_blur.clone());
                shell.capture_event();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            self.tree,
            layout,
            cursor,
            &self.viewport,
            renderer,
        )
    }

    fn index(&self) -> f32 {
        20.0
    }
}
