//! Right-click context menu overlay widget.
//!
//! Wraps a base element and shows a floating menu at the cursor position
//! when the user right-clicks on it.

use iced::advanced::layout::{self, Layout};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{self, tree, Tree, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::mouse;
use iced::{Element, Event, Length, Point, Rectangle, Size, Theme, Vector};

// ── Public API ──────────────────────────────────────────────────────

/// Wrap a base element with a right-click context menu.
///
/// `menu` is a closure that lazily builds the menu content each time
/// the menu is opened.
pub fn context_menu<'a, Message: 'a>(
    base: impl Into<Element<'a, Message>>,
    menu: impl Fn() -> Element<'a, Message> + 'a,
) -> Element<'a, Message> {
    ContextMenu {
        base: base.into(),
        menu: Box::new(menu),
    }
    .into()
}

// ── Internal types ──────────────────────────────────────────────────

struct ContextMenu<'a, Message> {
    base: Element<'a, Message>,
    menu: Box<dyn Fn() -> Element<'a, Message> + 'a>,
}

#[derive(Debug, Clone, Default)]
struct State {
    status: Status,
}

#[derive(Debug, Clone, Default)]
enum Status {
    #[default]
    Closed,
    Open {
        position: Point,
    },
}

// ── Widget impl ─────────────────────────────────────────────────────

impl<'a, Message: 'a> Widget<Message, Theme, iced::Renderer>
    for ContextMenu<'a, Message>
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        // Two children: [0] = base widget, [1] = menu content (placeholder).
        let menu_content = (self.menu)();
        vec![Tree::new(&self.base), Tree::new(&menu_content)]
    }

    fn diff(&self, tree: &mut Tree) {
        // Diff base child. Menu child gets diffed in overlay() when opened.
        tree.diff_children(std::slice::from_ref(&self.base));
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
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State>();

        // When menu is open, close on any left-click or Escape.
        if matches!(state.status, Status::Open { .. }) {
            match event {
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                    state.status = Status::Closed;
                    // Don't capture — let the click propagate to overlay buttons first.
                    return;
                }
                Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key:
                        iced::keyboard::Key::Named(
                            iced::keyboard::key::Named::Escape,
                        ),
                    ..
                }) => {
                    state.status = Status::Closed;
                    shell.capture_event();
                    return;
                }
                _ => return,
            }
        }

        match event {
            // Right-click on base -> open menu.
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                if let Some(position) = cursor.position_over(layout.bounds()) {
                    state.status = Status::Open { position };
                    shell.capture_event();
                    return;
                }
            }
            _ => {}
        }

        // Forward events to base when menu is closed.
        self.base.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.base.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        let state = tree.state.downcast_ref::<State>();

        match state.status {
            Status::Open { position } => {
                // Build fresh menu content and diff it against the persistent tree child.
                let content = (self.menu)();
                tree.children[1].diff(&content);

                Some(overlay::Element::new(Box::new(ContextMenuOverlay {
                    content,
                    tree: &mut tree.children[1],
                    position: position + translation,
                    viewport: *viewport,
                })))
            }
            Status::Closed => {
                // Delegate overlay to base widget.
                self.base.as_widget_mut().overlay(
                    &mut tree.children[0],
                    layout,
                    renderer,
                    viewport,
                    translation,
                )
            }
        }
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

impl<'a, Message: 'a> From<ContextMenu<'a, Message>> for Element<'a, Message> {
    fn from(cm: ContextMenu<'a, Message>) -> Self {
        Element::new(cm)
    }
}

// ── Overlay impl ────────────────────────────────────────────────────

struct ContextMenuOverlay<'a, Message> {
    content: Element<'a, Message>,
    tree: &'a mut Tree,
    position: Point,
    viewport: Rectangle,
}

const EDGE_PADDING: f32 = 5.0;

impl<'a, Message: 'a> overlay::Overlay<Message, Theme, iced::Renderer>
    for ContextMenuOverlay<'a, Message>
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

        let size = node.size();

        // Position at cursor, but clamp to viewport edges.
        let mut x = self.position.x;
        let mut y = self.position.y;

        if x + size.width > bounds.width - EDGE_PADDING {
            x = (bounds.width - size.width - EDGE_PADDING).max(EDGE_PADDING);
        }
        if y + size.height > bounds.height - EDGE_PADDING {
            y = (bounds.height - size.height - EDGE_PADDING).max(EDGE_PADDING);
        }

        node.move_to(Point::new(x, y))
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
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

        // Click outside overlay bounds -> close.
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            if cursor.position_over(layout.bounds()).is_none() {
                shell.capture_event();
            }
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
        10.0
    }
}

