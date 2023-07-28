use iced::advanced::widget::tree;
use iced::advanced::{layout, overlay, renderer, widget, Clipboard, Layout, Shell, Widget};
use iced::{event, mouse, Event, Length, Point, Rectangle, Size, Vector};

use super::{Element, Renderer};
use crate::Theme;

pub fn anchored_overlay<'a, Message: 'a>(
    base: impl Into<Element<'a, Message>>,
    overlay: impl Into<Element<'a, Message>>,
    anchor: Anchor,
    offset: f32,
) -> Element<'a, Message> {
    AnchoredOverlay {
        base: base.into(),
        overlay: overlay.into(),
        anchor,
        offset,
    }
    .into()
}

#[derive(Debug, Clone, Copy)]
pub enum Anchor {
    AboveTop,
    BelowTop,
}

struct AnchoredOverlay<'a, Message> {
    base: Element<'a, Message>,
    overlay: Element<'a, Message>,
    anchor: Anchor,
    offset: f32,
}

impl<'a, Message> Widget<Message, Renderer> for AnchoredOverlay<'a, Message> {
    fn width(&self) -> Length {
        self.base.as_widget().width()
    }

    fn height(&self) -> Length {
        self.base.as_widget().height()
    }

    fn layout(&self, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        self.base.as_widget().layout(renderer, limits)
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
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
        )
    }

    fn tag(&self) -> tree::Tag {
        struct Marker;
        tree::Tag::of::<Marker>()
    }

    fn state(&self) -> tree::State {
        tree::State::None
    }

    fn children(&self) -> Vec<widget::Tree> {
        vec![
            widget::Tree::new(&self.base),
            widget::Tree::new(&self.overlay),
        ]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[&self.base, &self.overlay]);
    }

    fn operate(
        &self,
        tree: &mut iced::advanced::widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation<Message>,
    ) {
        self.base
            .as_widget()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn on_event(
        &mut self,
        tree: &mut widget::Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {
        self.base.as_widget_mut().on_event(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        )
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
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
        tree: &'b mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'b, Message, Renderer>> {
        let (first, second) = tree.children.split_at_mut(1);

        let base = self
            .base
            .as_widget_mut()
            .overlay(&mut first[0], layout, renderer);

        let overlay = overlay::Element::new(
            layout.position(),
            Box::new(Overlay {
                content: &mut self.overlay,
                tree: &mut second[0],
                anchor: self.anchor,
                offset: self.offset,
                base_layout: layout.bounds(),
            }),
        );

        Some(
            overlay::Group::with_children(base.into_iter().chain(Some(overlay)).collect())
                .overlay(),
        )
    }
}

impl<'a, Message> From<AnchoredOverlay<'a, Message>> for Element<'a, Message>
where
    Message: 'a,
{
    fn from(anchored_overlay: AnchoredOverlay<'a, Message>) -> Self {
        Element::new(anchored_overlay)
    }
}

struct Overlay<'a, 'b, Message> {
    content: &'b mut Element<'a, Message>,
    tree: &'b mut widget::Tree,
    anchor: Anchor,
    offset: f32,
    base_layout: Rectangle,
}

impl<'a, 'b, Message> overlay::Overlay<Message, Renderer> for Overlay<'a, 'b, Message> {
    fn layout(&self, renderer: &Renderer, bounds: Size, position: Point) -> layout::Node {
        let height = match self.anchor {
            // From top of base to top of viewport
            Anchor::AboveTop => position.y,
            // From top of base to bottom of viewport
            Anchor::BelowTop => bounds.height - position.y,
        };

        let limits = layout::Limits::new(
            Size::ZERO,
            Size {
                width: self.base_layout.width,
                height,
            },
        )
        .width(Length::Fill)
        .height(Length::Fill);

        let mut node = self.content.as_widget().layout(renderer, &limits);

        let translation = match self.anchor {
            // Overlay height + offset above the top
            Anchor::AboveTop => Vector::new(0.0, -(node.size().height + self.offset)),
            // Offset below the top
            Anchor::BelowTop => Vector::new(0.0, self.offset),
        };

        node.move_to(position + translation);

        node
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
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
            &layout.bounds(),
        );
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation<Message>,
    ) {
        self.content
            .as_widget_mut()
            .operate(self.tree, layout, renderer, operation);
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        self.content.as_widget_mut().on_event(
            self.tree,
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        )
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> iced::advanced::mouse::Interaction {
        self.content
            .as_widget()
            .mouse_interaction(self.tree, layout, cursor, viewport, renderer)
    }

    fn is_over(&self, layout: Layout<'_>, _renderer: &Renderer, cursor_position: Point) -> bool {
        layout.bounds().contains(cursor_position)
    }
}
