//! # Network Sprite
//!
//! The network service for AethelOS.
//! Connections to other realms are not mere data streams -
//! they are bridges between living systems.
//!
//! ## Philosophy
//! The Network Sprite does not force connections.
//! It establishes pathways, allowing data to flow naturally
//! between harmonious systems.

#![no_std]

extern crate alloc;

use alloc::vec::Vec;

/// A network connection
pub struct Connection {
    pub id: ConnectionId,
    pub remote_realm: &'static str,
    pub state: ConnectionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Establishing,
    Connected,
    Flowing,  // Active data transfer
    Resting,  // Idle but connected
    Fading,   // Closing
}

/// The Network Sprite service
pub struct NetworkSprite {
    connections: Vec<Connection>,
    next_id: u64,
}

impl Default for NetworkSprite {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkSprite {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            next_id: 1,
        }
    }

    /// Establish a connection to another realm
    pub fn connect(&mut self, realm: &'static str) -> ConnectionId {
        let id = ConnectionId(self.next_id);
        self.next_id += 1;

        let connection = Connection {
            id,
            remote_realm: realm,
            state: ConnectionState::Establishing,
        };

        self.connections.push(connection);

        id
    }

    /// Send data through a connection
    pub fn send(&mut self, id: ConnectionId, data: &[u8]) -> Result<(), NetworkError> {
        let connection = self
            .connections
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or(NetworkError::ConnectionNotFound)?;

        if connection.state != ConnectionState::Connected
            && connection.state != ConnectionState::Flowing
        {
            return Err(NetworkError::NotConnected);
        }

        connection.state = ConnectionState::Flowing;

        // In a real implementation, send data through network stack

        Ok(())
    }

    /// Receive data from a connection
    pub fn receive(&mut self, id: ConnectionId) -> Result<Vec<u8>, NetworkError> {
        let connection = self
            .connections
            .iter()
            .find(|c| c.id == id)
            .ok_or(NetworkError::ConnectionNotFound)?;

        if connection.state != ConnectionState::Connected
            && connection.state != ConnectionState::Flowing
        {
            return Err(NetworkError::NotConnected);
        }

        // In a real implementation, receive data from network stack

        Ok(Vec::new())
    }

    /// Close a connection
    pub fn close(&mut self, id: ConnectionId) {
        if let Some(connection) = self.connections.iter_mut().find(|c| c.id == id) {
            connection.state = ConnectionState::Fading;
        }

        // Eventually remove from list
        self.connections.retain(|c| c.id != id);
    }
}

#[derive(Debug)]
pub enum NetworkError {
    ConnectionNotFound,
    NotConnected,
    Timeout,
    RealmUnreachable,
}
