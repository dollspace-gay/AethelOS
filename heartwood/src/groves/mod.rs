//! # Groves - Ring 1 Service Management
//!
//! The Groves are privileged services that operate in Ring 1,
//! providing system functionality with elevated privileges but
//! isolated from the kernel proper.
//!
//! ## Philosophy
//! Groves are the sacred spaces between kernel and userland—
//! neither fully omnipotent nor powerless. They hold the ancient
//! knowledge needed for specific domains: filesystem access,
//! graphics rendering, network communication.
//!
//! ## Architecture
//! - **GroveManager**: Tracks all loaded services, their state, and capabilities
//! - **ServiceRegistry**: Maps service names to their implementations
//! - **Capability System**: Controls what each Grove can access
//!
//! ## Known Groves
//! - **World-Tree**: Query-based filesystem with versioning
//! - **The Weave**: Vector scene graph compositor
//! - **Lanthir**: Window manager
//! - **Network Sprite**: TCP/IP stack and network protocols

pub mod manager;
pub mod service;
pub mod lifecycle;

pub use manager::{GroveManager, GroveError};
pub use service::{ServiceId, ServiceInfo, ServiceState, ServiceCapability, ResourceLimits, RespawnPolicy};
pub use lifecycle::{
    load_service, start_service, stop_service, restart_service,
    handle_service_crash, list_services,
    ServiceConfig,
};
