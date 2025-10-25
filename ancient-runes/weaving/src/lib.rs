//! # Weaving - The Weave API
//!
//! The toolkit for building graphical applications in AethelOS.
//! Applications do not draw pixels; they weave intentions
//! into the living tapestry of the Weave.
//!
//! ## Philosophy
//! Weaving is not about control; it's about expression.
//! You describe what should be, and the Weave makes it so.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use alloc::boxed::Box;

/// A widget in the Weave
pub trait Widget {
    /// Get the natural size of this widget
    fn natural_size(&self) -> (f32, f32);

    /// Render this widget to the scene graph
    fn render(&self) -> WidgetNode;

    /// Handle an event
    fn handle_event(&mut self, event: Event);
}

/// A node representing a widget in the scene graph
pub struct WidgetNode {
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub children: Vec<WidgetNode>,
}

/// Events that can be sent to widgets
#[derive(Debug, Clone, Copy)]
pub enum Event {
    MouseMove { x: f32, y: f32 },
    MouseDown { button: MouseButton },
    MouseUp { button: MouseButton },
    KeyPress { key: Key },
    KeyRelease { key: Key },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    A, B, C, // ... etc
    Enter,
    Escape,
    Space,
}

/// A simple button widget
pub struct Button {
    pub text: &'static str,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub on_click: Option<fn()>,
}

impl Widget for Button {
    fn natural_size(&self) -> (f32, f32) {
        self.size
    }

    fn render(&self) -> WidgetNode {
        WidgetNode {
            position: self.position,
            size: self.size,
            children: Vec::new(),
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::MouseDown { button: MouseButton::Left } => {
                if let Some(callback) = self.on_click {
                    callback();
                }
            }
            _ => {}
        }
    }
}

/// A text label widget
pub struct Label {
    pub text: &'static str,
    pub position: (f32, f32),
    pub font_size: f32,
}

impl Widget for Label {
    fn natural_size(&self) -> (f32, f32) {
        let width = self.text.len() as f32 * self.font_size * 0.6;
        (width, self.font_size)
    }

    fn render(&self) -> WidgetNode {
        WidgetNode {
            position: self.position,
            size: self.natural_size(),
            children: Vec::new(),
        }
    }

    fn handle_event(&mut self, _event: Event) {
        // Labels don't handle events
    }
}

/// A container that arranges children vertically
pub struct VBox {
    pub position: (f32, f32),
    pub children: Vec<Box<dyn Widget>>,
    pub spacing: f32,
}

impl Widget for VBox {
    fn natural_size(&self) -> (f32, f32) {
        let mut width = 0.0f32;
        let mut height = 0.0f32;

        for child in &self.children {
            let (w, h) = child.natural_size();
            width = width.max(w);
            height += h + self.spacing;
        }

        (width, height)
    }

    fn render(&self) -> WidgetNode {
        WidgetNode {
            position: self.position,
            size: self.natural_size(),
            children: Vec::new(), // Would contain rendered children
        }
    }

    fn handle_event(&mut self, event: Event) {
        // Propagate to children
        for child in &mut self.children {
            child.handle_event(event);
        }
    }
}
