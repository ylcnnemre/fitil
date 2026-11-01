//! A button's menu opening beside it, over the rest of the window, like the menus of the
//! tool groups in TradingView's drawing bar.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Tree, Widget};
use iced::advanced::{Clipboard, Shell, mouse, overlay, renderer};
use iced::{Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector, touch};

/// Pixels between the button and its menu, and kept free at the window's edges.
const GAP: f32 = 6.0;

/// `anchor` with `menu` on its right while there is a menu. A press anywhere but on the
/// menu closes it with `on_close` and goes no further, so that closing the menu does
/// nothing else, such as placing a drawing.
pub struct Flyout<'a, Message> {
    anchor: Element<'a, Message>,
    menu: Option<Element<'a, Message>>,
    on_close: Message,
}

pub fn flyout<'a, Message>(
    anchor: impl Into<Element<'a, Message>>,
    menu: Option<Element<'a, Message>>,
    on_close: Message,
) -> Flyout<'a, Message> {
    Flyout {
        anchor: anchor.into(),
        menu,
        on_close,
    }
}

impl<Message> Flyout<'_, Message> {
    fn elements(&self) -> impl Iterator<Item = &dyn Widget<Message, Theme, Renderer>> {
        std::iter::once(&self.anchor)
            .chain(&self.menu)
            .map(Element::as_widget)
    }
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for Flyout<'_, Message> {
    fn children(&self) -> Vec<Tree> {
        self.elements().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        let elements: Vec<_> = self.elements().collect();
        tree.diff_children(&elements);
    }

    fn size(&self) -> Size<Length> {
        self.anchor.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.anchor.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.anchor
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.anchor.as_widget_mut().update(
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

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.anchor.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.anchor.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        self.anchor
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let Flyout {
            anchor,
            menu,
            on_close,
        } = self;
        let (anchor_tree, menu_tree) = match tree.children.as_mut_slice() {
            [anchor, menu] => (anchor, Some(menu)),
            [anchor] => (anchor, None),
            _ => return None,
        };
        // The anchor's own overlays, such as a tooltip, and the menu.
        let overlays: Vec<_> = anchor
            .as_widget_mut()
            .overlay(anchor_tree, layout, renderer, viewport, translation)
            .into_iter()
            .chain(menu.as_mut().zip(menu_tree).map(|(menu, tree)| {
                overlay::Element::new(Box::new(Menu {
                    menu,
                    tree,
                    anchor: layout.bounds() + translation,
                    on_close,
                }))
            }))
            .collect();
        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

impl<'a, Message: Clone + 'a> From<Flyout<'a, Message>> for Element<'a, Message> {
    fn from(flyout: Flyout<'a, Message>) -> Self {
        Element::new(flyout)
    }
}

/// The open menu, over everything else.
struct Menu<'a, 'b, Message> {
    menu: &'b mut Element<'a, Message>,
    tree: &'b mut Tree,
    /// Where the menu's button is in the window.
    anchor: Rectangle,
    on_close: &'b Message,
}

impl<Message: Clone> overlay::Overlay<Message, Theme, Renderer> for Menu<'_, '_, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let x = self.anchor.x + self.anchor.width + GAP;
        let room = Size::new(
            (bounds.width - x - GAP).max(0.0),
            (bounds.height - 2.0 * GAP).max(0.0),
        );
        let node = self.menu.as_widget_mut().layout(
            self.tree,
            renderer,
            &layout::Limits::new(Size::ZERO, room),
        );
        // Level with the button, or higher where it would run past the window's bottom.
        let y = self
            .anchor
            .y
            .min(bounds.height - GAP - node.size().height)
            .max(GAP);
        node.move_to(Point::new(x, y))
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.menu.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout,
            cursor,
            &layout.bounds(),
        );
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) {
        let bounds = layout.bounds();
        self.menu.as_widget_mut().update(
            self.tree, event, layout, cursor, renderer, clipboard, shell, &bounds,
        );
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(_))
            | Event::Touch(touch::Event::FingerPressed { .. }) => {
                if !cursor.is_over(bounds) {
                    shell.publish(self.on_close.clone());
                }
                shell.capture_event();
            }
            // Scrolling over the menu is not for the chart under it.
            Event::Mouse(mouse::Event::WheelScrolled { .. }) if cursor.is_over(bounds) => {
                shell.capture_event();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();
        if !cursor.is_over(bounds) {
            return mouse::Interaction::None;
        }
        // Anything but none keeps the cursor from the widgets under the menu.
        self.menu
            .as_widget()
            .mouse_interaction(self.tree, layout, cursor, &bounds, renderer)
            .max(mouse::Interaction::Idle)
    }
}
