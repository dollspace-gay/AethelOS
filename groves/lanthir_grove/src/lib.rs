//! # Lanthir Grove
//!
//! The window management service for AethelOS.
//! Windows are not mere containers - they are living entities
//! with purpose and relationships.
//!
//! ## Philosophy
//! Lanthir does not stack windows mechanically.
//! It arranges them harmoniously, respecting their essence
//! and the user's intent.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;

/// A window managed by Lanthir
pub struct Window {
    pub id: WindowId,
    pub title: &'static str,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub z_order: i32,
    pub state: WindowState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Fading,  // Closing animation
}

/// The Lanthir window manager
pub struct Lanthir {
    windows: Vec<Window>,
    focused: Option<WindowId>,
    next_id: u64,
}

impl Default for Lanthir {
    fn default() -> Self {
        Self::new()
    }
}

impl Lanthir {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            focused: None,
            next_id: 1,
        }
    }

    /// Create a new window
    pub fn create_window(&mut self, title: &'static str, position: (f32, f32), size: (f32, f32)) -> WindowId {
        let id = WindowId(self.next_id);
        self.next_id += 1;

        let window = Window {
            id,
            title,
            position,
            size,
            z_order: self.windows.len() as i32,
            state: WindowState::Normal,
        };

        self.windows.push(window);
        self.focused = Some(id);

        id
    }

    /// Focus a window
    pub fn focus(&mut self, id: WindowId) {
        self.focused = Some(id);

        // Bring to front
        let new_z_order = self.windows.len() as i32;
        if let Some(window) = self.windows.iter_mut().find(|w| w.id == id) {
            window.z_order = new_z_order;
        }
    }

    /// Close a window
    pub fn close(&mut self, id: WindowId) {
        self.windows.retain(|w| w.id != id);

        if self.focused == Some(id) {
            self.focused = self.windows.last().map(|w| w.id);
        }
    }
}
