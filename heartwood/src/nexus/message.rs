//! Message definitions - The Astral Packets of the Nexus

use alloc::vec::Vec;

/// A message is the fundamental unit of communication in AethelOS
#[derive(Debug, Clone)]
pub struct Message {
    /// The type and payload of the message
    pub message_type: MessageType,

    /// Priority for harmony-based routing
    pub priority: MessagePriority,

    /// The sender's capability (for replies)
    pub reply_to: Option<u64>,
}

impl Message {
    pub fn new(message_type: MessageType, priority: MessagePriority) -> Self {
        Self {
            message_type,
            priority,
            reply_to: None,
        }
    }

    pub fn with_reply(mut self, reply_capability: u64) -> Self {
        self.reply_to = Some(reply_capability);
        self
    }
}

/// The type and payload of a message
#[derive(Debug, Clone)]
pub enum MessageType {
    /// A request to allocate resources
    ResourceRequest {
        resource_type: ResourceType,
        amount: usize,
    },

    /// A grant of resources
    ResourceGrant {
        resource_type: ResourceType,
        handle: u64,
    },

    /// A signal indicating state change
    Signal {
        signal_type: SignalType,
    },

    /// Data transfer
    Data {
        payload: Vec<u8>,
    },

    /// A query for system information
    Query {
        query_type: QueryType,
    },

    /// Response to a query
    Response {
        data: Vec<u8>,
    },

    /// Notification of disharmony
    DisharmonyAlert {
        severity: u8,
        description: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Memory,
    CpuTime,
    FileHandle,
    ChannelCapability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalType {
    ThreadWeaving,    // Thread started
    ThreadResting,    // Thread idle
    ThreadTangled,    // Thread blocked/error
    ThreadFading,     // Thread exiting
    SystemHarmony,    // System in good state
    SystemDisharmony, // System under stress
    ServiceShutdown,  // Request service to shut down gracefully
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    SystemStatus,
    ResourceAvailability,
    ThreadState,
    MemoryUsage,
}

/// Priority levels for message delivery
/// Lower numeric values = higher priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    Critical = 0,  // System-critical messages
    High = 1,      // Important user-facing operations
    Normal = 2,    // Standard operations
    Low = 3,       // Background tasks
    Idle = 4,      // Lowest priority
}
