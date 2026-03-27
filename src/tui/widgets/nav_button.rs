use std::any::Any;
use std::cell::Cell;

use textual_rs::widget::button::messages::Pressed;
use textual_rs::widget::context::AppContext;
use textual_rs::widget::EventPropagation;
use textual_rs::{Button, Widget, WidgetId};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

pub mod messages {
    use textual_rs::event::message::Message;

    /// Emitted by NavButton when its inner Button is pressed (Enter/Space/click).
    /// Carries the action string so a parent can distinguish multiple NavButtons.
    pub struct Activated {
        pub action: String,
    }

    impl Message for Activated {}
}

/// A `Button` wrapper that re-emits a typed [`messages::Activated`] message on press.
///
/// Because all Buttons emit the generic `button::messages::Pressed`, a parent with
/// multiple buttons cannot tell which was pressed. NavButton catches `Pressed` from
/// its inner Button and re-emits `Activated { action }` with a caller-supplied action
/// name, letting the parent dispatch via `on_action` or match in `on_event`.
pub struct NavButton {
    label: String,
    action: String,
    own_id: Cell<Option<WidgetId>>,
}

impl NavButton {
    pub fn new(label: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            action: action.into(),
            own_id: Cell::new(None),
        }
    }
}

impl Widget for NavButton {
    fn widget_type_name(&self) -> &'static str {
        "NavButton"
    }

    fn on_mount(&self, id: WidgetId) {
        self.own_id.set(Some(id));
    }

    fn on_unmount(&self, _id: WidgetId) {
        self.own_id.set(None);
    }

    fn compose(&self) -> Vec<Box<dyn Widget>> {
        vec![Box::new(Button::new(self.label.clone()))]
    }

    fn on_event(&self, event: &dyn Any, ctx: &AppContext) -> EventPropagation {
        if event.downcast_ref::<Pressed>().is_some() {
            if let Some(id) = self.own_id.get() {
                ctx.post_message(id, messages::Activated { action: self.action.clone() });
            }
            return EventPropagation::Stop;
        }
        EventPropagation::Continue
    }

    fn render(&self, _ctx: &AppContext, _area: Rect, _buf: &mut Buffer) {}
}
