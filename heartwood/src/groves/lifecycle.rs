//! Service lifecycle management
//!
//! Handles loading, starting, stopping, and restarting Ring 1 services.
//! Implements crash detection and automatic respawn policies.

use super::manager::{get_grove_manager, GroveError};
use super::service::{ServiceId, ServiceState, ServiceCapability, ResourceLimits, RespawnPolicy};
use crate::loom_of_fate::{self, ThreadId, ThreadPriority, VesselId};
use alloc::string::String;
use alloc::vec::Vec;

/// Configuration for loading a service
pub struct ServiceConfig {
    /// Service name (e.g., "world-tree")
    pub name: String,

    /// Entry point address (from ELF loading)
    pub entry_point: u64,

    /// Stack top address (pre-allocated)
    pub stack_top: u64,

    /// Page table physical address (from Vessel)
    pub page_table_phys: u64,

    /// Thread priority
    pub priority: ThreadPriority,

    /// Capabilities to grant
    pub capabilities: Vec<ServiceCapability>,

    /// Resource limits
    pub limits: ResourceLimits,

    /// Respawn policy if service crashes
    pub respawn_policy: RespawnPolicy,
}

/// Load a service into memory and register it, but don't start it yet
///
/// This function:
/// 1. Creates a new Vessel (address space) for the service
/// 2. Loads the service's ELF binary into the Vessel's memory
/// 3. Registers the service with the Grove Manager
/// 4. Returns the ServiceId
///
/// The service remains in Loading state until start_service() is called.
///
/// # Arguments
/// * `config` - Service configuration including name, entry point, capabilities
///
/// # Returns
/// * `Ok(ServiceId)` - The ID of the loaded service
/// * `Err(GroveError)` - If loading fails
pub fn load_service(config: ServiceConfig) -> Result<ServiceId, GroveError> {
    crate::serial_println!("[Lifecycle] Loading service '{}'...", config.name);

    // Create a temporary ThreadId for the main thread
    // (we'll create the actual thread after we have the Vessel)
    let temp_thread_id = ThreadId(0);

    // Create a new Vessel for this service in the Harbor
    // NOTE: Currently using moor_vessel() which creates a basic Vessel
    // In a full implementation, this would use moor_service_vessel() with ELF loading
    let vessel_id = {
        let mut harbor = loom_of_fate::get_harbor().lock();
        harbor.moor_vessel(
            None, // No parent for top-level services
            config.page_table_phys,
            config.stack_top, // Use stack top as kernel_stack for now
            temp_thread_id,
            config.name.clone(), // Use service name as "fate" (RBAC role)
        )
    };

    crate::serial_println!("[Lifecycle] Created Vessel {:?} for service '{}'", vessel_id, config.name);

    // Create the main service thread
    let thread_id = loom_of_fate::create_service_thread(
        vessel_id,
        config.entry_point,
        config.stack_top,
        config.priority,
    ).map_err(|_| GroveError::InvalidState)?;

    // Update the Vessel's main_thread to the actual thread we just created
    {
        let mut harbor = loom_of_fate::get_harbor().lock();
        if let Some(vessel) = harbor.find_vessel_mut(vessel_id) {
            vessel.main_thread = thread_id;
        }
    }

    // Register with Grove Manager
    let mut manager = get_grove_manager().lock();
    let service_id = manager.register_service(
        config.name.clone(),
        vessel_id,
        thread_id,
        config.capabilities,
        config.limits,
        config.respawn_policy,
    )?;

    crate::serial_println!("[Lifecycle] Service '{}' loaded (ID: {:?})", config.name, service_id);

    Ok(service_id)
}

/// Start a loaded service
///
/// Transitions the service from Loading -> Running state and schedules
/// its main thread for execution.
///
/// # Arguments
/// * `service_id` - The ID of the service to start
///
/// # Returns
/// * `Ok(())` - Service started successfully
/// * `Err(GroveError)` - If service not found or in wrong state
pub fn start_service(service_id: ServiceId) -> Result<(), GroveError> {
    let mut manager = get_grove_manager().lock();

    let service = manager.find_service(service_id)
        .ok_or(GroveError::ServiceNotFound)?;

    if service.state != ServiceState::Loading {
        return Err(GroveError::InvalidState);
    }

    let name = service.name.clone();
    let thread_id = service.main_thread_id;

    // Mark service as running
    manager.set_service_running(service_id)?;

    // The thread is already in the Loom's ready queue from create_service_thread()
    // so we don't need to schedule it again

    crate::serial_println!("[Lifecycle] Service '{}' started (thread {:?})", name, thread_id);

    Ok(())
}

/// Stop a service gracefully
///
/// Sends a stop signal to the service and waits for it to terminate.
/// If the service doesn't stop within a timeout, it will be forcibly killed.
///
/// # Arguments
/// * `service_id` - The ID of the service to stop
///
/// # Returns
/// * `Ok(())` - Service stopped successfully
/// * `Err(GroveError)` - If service not found or already stopped
pub fn stop_service(service_id: ServiceId) -> Result<(), GroveError> {
    let (name, vessel_id, thread_id) = {
        let mut manager = get_grove_manager().lock();

        let service = manager.find_service(service_id)
            .ok_or(GroveError::ServiceNotFound)?;

        if service.state == ServiceState::Stopped {
            return Err(GroveError::InvalidState);
        }

        let name = service.name.clone();
        let vessel_id = service.vessel_id;
        let thread_id = service.main_thread_id;

        // Mark service as stopped immediately
        manager.set_service_stopped(service_id)?;

        (name, vessel_id, thread_id)
    };

    crate::serial_println!("[Lifecycle] Stopping service '{}' (thread {:?})...", name, thread_id);

    // Send IPC shutdown signal to service (if it has a registered channel)
    // TODO: Implement service channel registration and lookup
    // For now, we log the intent and proceed with forcible termination
    crate::serial_println!("[Lifecycle] Would send ServiceShutdown signal via Nexus (not yet implemented)");

    // In a full implementation, this would be:
    // if let Some(channel_id) = get_service_channel(vessel_id) {
    //     let shutdown_msg = Message::new(
    //         MessageType::Signal { signal_type: SignalType::ServiceShutdown },
    //         MessagePriority::High
    //     );
    //     crate::nexus::send(channel_id, shutdown_msg)?;
    //
    //     // Wait for service to exit gracefully (with timeout)
    //     for _ in 0..100 {  // ~1 second timeout
    //         if service_has_exited(service_id) {
    //             break;
    //         }
    //         crate::attunement::timer::delay_ms(10);
    //     }
    // }

    // For now, we forcibly terminate

    // Clean up the service's threads
    // NOTE: In Phase 3+, this would iterate all threads in the Vessel
    // For now, we just have the main thread
    crate::serial_println!("[Lifecycle] Terminating main thread {:?}", thread_id);
    if let Err(e) = loom_of_fate::terminate_thread(thread_id) {
        crate::serial_println!("[Lifecycle] Warning: Failed to terminate thread {:?}: {:?}", thread_id, e);
    }

    // Clean up the Vessel and its memory
    crate::serial_println!("[Lifecycle] Unmooring Vessel {:?}", vessel_id);
    {
        let mut harbor = loom_of_fate::get_harbor().lock();
        harbor.unmoor_vessel(vessel_id);
    }

    crate::serial_println!("[Lifecycle] Service '{}' stopped and cleaned up", name);

    Ok(())
}

/// Restart a service
///
/// Convenience function that stops and then starts a service.
///
/// # Arguments
/// * `service_id` - The ID of the service to restart
///
/// # Returns
/// * `Ok(())` - Service restarted successfully
/// * `Err(GroveError)` - If restart fails
pub fn restart_service(service_id: ServiceId) -> Result<(), GroveError> {
    let name = {
        let manager = get_grove_manager().lock();
        let service = manager.find_service(service_id)
            .ok_or(GroveError::ServiceNotFound)?;
        service.name.clone()
    };

    crate::serial_println!("[Lifecycle] Restarting service '{}'...", name);

    stop_service(service_id)?;
    start_service(service_id)?;

    crate::serial_println!("[Lifecycle] Service '{}' restarted", name);

    Ok(())
}

/// Handle a service crash
///
/// Called by the fault handler when a service encounters an error.
/// Checks the respawn policy and decides whether to restart the service.
///
/// # Arguments
/// * `service_id` - The ID of the crashed service
/// * `fault_reason` - Description of what caused the crash
///
/// # Returns
/// * `Ok(bool)` - true if service was respawned, false otherwise
/// * `Err(GroveError)` - If error handling fails
pub fn handle_service_crash(
    service_id: ServiceId,
    fault_reason: &str,
) -> Result<bool, GroveError> {
    crate::serial_println!(
        "[Lifecycle] SERVICE CRASH: {:?} faulted: {}",
        service_id,
        fault_reason
    );

    // Get current tick count for respawn tracking
    let current_time = crate::attunement::timer::ticks();

    let (name, should_respawn) = {
        let mut manager = get_grove_manager().lock();

        let service = manager.find_service_mut(service_id)
            .ok_or(GroveError::ServiceNotFound)?;

        let name = service.name.clone();

        // Check if service should be respawned
        let should_respawn = service.should_respawn(current_time);

        // Mark as faulted
        manager.set_service_faulted(service_id)?;

        (name, should_respawn)
    };

    if should_respawn {
        crate::serial_println!(
            "[Lifecycle] Service '{}' will be respawned (policy allows it)",
            name
        );

        // Attempt to restart the service
        match restart_service(service_id) {
            Ok(()) => {
                crate::serial_println!(
                    "[Lifecycle] Service '{}' respawned successfully",
                    name
                );
                Ok(true)
            }
            Err(e) => {
                crate::serial_println!(
                    "[Lifecycle] CRITICAL: Failed to respawn service '{}': {:?}",
                    name,
                    e
                );
                Ok(false)
            }
        }
    } else {
        crate::serial_println!(
            "[Lifecycle] Service '{}' will NOT be respawned (policy limit reached)",
            name
        );
        Ok(false)
    }
}

/// List all services and their states
///
/// Useful for debugging and status monitoring.
pub fn list_services() {
    let manager = get_grove_manager().lock();

    crate::println!("◈ Registered Services:");

    let count = manager.service_count();
    if count == 0 {
        crate::println!("  (none)");
        return;
    }

    for service in manager.all_services() {
        let state_str = match service.state {
            ServiceState::Loading => "Loading",
            ServiceState::Running => "Running",
            ServiceState::Stopped => "Stopped",
            ServiceState::Faulted => "FAULTED",
        };

        crate::println!(
            "  [{:?}] {} - {} (Vessel {:?}, Thread {:?})",
            service.id,
            service.name,
            state_str,
            service.vessel_id,
            service.main_thread_id
        );

        crate::println!(
            "    Resources: {} KB memory, {} threads",
            service.memory_used / 1024,
            service.thread_count
        );
    }
}
