//! Service state tracking and metadata

use crate::loom_of_fate::{ThreadId, VesselId};
use alloc::string::String;
use alloc::vec::Vec;

/// Unique identifier for a Grove service
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ServiceId(pub u64);

/// Respawn policy for a service that crashes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RespawnPolicy {
    /// Never respawn, even on crash
    Never,

    /// Always respawn, regardless of exit reason
    Always,

    /// Only respawn on abnormal termination (crash)
    OnFailure,

    /// Respawn up to N times, then give up
    Limited(u32),
}

/// The lifecycle state of a Grove service
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    /// Service is in the process of loading (ELF parsing, memory setup)
    Loading,

    /// Service is running and accepting requests
    Running,

    /// Service has been stopped gracefully
    Stopped,

    /// Service has encountered a fault (page fault, illegal instruction, etc.)
    Faulted,
}

/// Capabilities that can be granted to a Grove service
///
/// Services start with NO capabilities and must explicitly request them.
/// The kernel validates and grants capabilities based on service identity
/// and security policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceCapability {
    /// Access to physical memory regions (for hardware drivers)
    PhysicalMemory,

    /// Access to I/O ports (for hardware drivers)
    IoPort,

    /// Ability to create new threads
    ThreadCreate,

    /// Ability to manage memory allocations
    MemoryManage,

    /// Access to the filesystem (World-Tree)
    Filesystem,

    /// Access to graphics hardware (VGA, framebuffer)
    Graphics,

    /// Access to network hardware
    Network,

    /// Ability to send IPC messages to other services
    IpcSend,

    /// Ability to receive IPC messages from other services
    IpcReceive,
}

/// Resource limits for a Grove service
///
/// These limits prevent a misbehaving service from consuming
/// all system resources.
#[derive(Debug, Clone, Copy)]
pub struct ResourceLimits {
    /// Maximum memory the service can allocate (bytes)
    pub max_memory: usize,

    /// Maximum number of threads the service can create
    pub max_threads: usize,

    /// Maximum CPU time per scheduling period (percentage, 0-100)
    pub max_cpu_percent: u8,

    /// Maximum number of open file handles (for World-Tree)
    pub max_file_handles: usize,

    /// Maximum number of IPC messages per second
    pub max_ipc_rate: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory: 64 * 1024 * 1024,  // 64 MB default
            max_threads: 16,
            max_cpu_percent: 50,  // 50% CPU time
            max_file_handles: 256,
            max_ipc_rate: 10000,  // 10k messages/sec
        }
    }
}

/// Information about a registered Grove service
pub struct ServiceInfo {
    /// Unique service identifier
    pub id: ServiceId,

    /// Human-readable service name (e.g., "world-tree", "the-weave")
    pub name: String,

    /// Current lifecycle state
    pub state: ServiceState,

    /// The Vessel (process) this service runs in
    pub vessel_id: VesselId,

    /// The main service thread
    pub main_thread_id: ThreadId,

    /// Capabilities granted to this service
    pub capabilities: Vec<ServiceCapability>,

    /// Resource limits for this service
    pub limits: ResourceLimits,

    /// Total CPU time consumed (nanoseconds)
    pub cpu_time_used: u64,

    /// Total memory currently allocated (bytes)
    pub memory_used: usize,

    /// Number of threads currently active
    pub thread_count: usize,

    /// Number of IPC messages sent
    pub ipc_messages_sent: u64,

    /// Number of IPC messages received
    pub ipc_messages_received: u64,

    /// Respawn policy for this service
    pub respawn_policy: RespawnPolicy,

    /// Number of times this service has crashed and been respawned
    pub crash_count: u32,

    /// Tick count of the last crash (for rate limiting respawns)
    pub last_crash_time: u64,
}

impl ServiceInfo {
    /// Create a new ServiceInfo for a loading service
    pub fn new(
        id: ServiceId,
        name: String,
        vessel_id: VesselId,
        main_thread_id: ThreadId,
        capabilities: Vec<ServiceCapability>,
        limits: ResourceLimits,
        respawn_policy: RespawnPolicy,
    ) -> Self {
        Self {
            id,
            name,
            state: ServiceState::Loading,
            vessel_id,
            main_thread_id,
            capabilities,
            limits,
            cpu_time_used: 0,
            memory_used: 0,
            thread_count: 1,  // Main thread
            ipc_messages_sent: 0,
            ipc_messages_received: 0,
            respawn_policy,
            crash_count: 0,
            last_crash_time: 0,
        }
    }

    /// Check if this service has a specific capability
    pub fn has_capability(&self, cap: ServiceCapability) -> bool {
        self.capabilities.contains(&cap)
    }

    /// Check if this service is within its resource limits
    pub fn within_limits(&self) -> bool {
        self.memory_used <= self.limits.max_memory
            && self.thread_count <= self.limits.max_threads
    }

    /// Mark the service as running
    pub fn set_running(&mut self) {
        self.state = ServiceState::Running;
    }

    /// Mark the service as faulted
    pub fn set_faulted(&mut self) {
        self.state = ServiceState::Faulted;
    }

    /// Mark the service as stopped
    pub fn set_stopped(&mut self) {
        self.state = ServiceState::Stopped;
    }

    /// Check if this service should be respawned after a crash
    ///
    /// Updates crash_count and last_crash_time if respawn is allowed.
    ///
    /// # Arguments
    /// * `current_time` - Current tick count
    ///
    /// # Returns
    /// * `true` - Service should be respawned
    /// * `false` - Service should not be respawned (policy limit reached)
    pub fn should_respawn(&mut self, current_time: u64) -> bool {
        match self.respawn_policy {
            RespawnPolicy::Never => false,
            RespawnPolicy::Always => {
                self.crash_count += 1;
                self.last_crash_time = current_time;
                true
            }
            RespawnPolicy::OnFailure => {
                self.crash_count += 1;
                self.last_crash_time = current_time;
                true
            }
            RespawnPolicy::Limited(max) => {
                if self.crash_count >= max {
                    false
                } else {
                    self.crash_count += 1;
                    self.last_crash_time = current_time;
                    true
                }
            }
        }
    }
}
