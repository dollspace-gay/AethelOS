//! Grove Manager - Service Registry and Lifecycle Management

use super::service::{ServiceId, ServiceInfo, ServiceState, ServiceCapability, ResourceLimits};
use crate::loom_of_fate::{ThreadId, VesselId};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;

/// Errors that can occur during Grove management
#[derive(Debug)]
pub enum GroveError {
    /// Service with this ID already exists
    ServiceAlreadyExists,

    /// Service with this ID not found
    ServiceNotFound,

    /// Service name is invalid or already taken
    InvalidServiceName,

    /// Service is in the wrong state for this operation
    InvalidState,

    /// Service has exceeded its resource limits
    ResourceLimitExceeded,

    /// Service does not have the required capability
    CapabilityDenied,

    /// Maximum number of services reached
    TooManyServices,
}

/// Maximum number of Grove services that can be loaded
const MAX_SERVICES: usize = 64;

/// The Grove Manager - Registry of all Ring 1 services
///
/// This is the kernel's view of all privileged services. It tracks
/// their state, capabilities, resource usage, and provides methods
/// for lifecycle management.
pub struct GroveManager {
    /// Map of ServiceId -> ServiceInfo
    services: BTreeMap<ServiceId, ServiceInfo>,

    /// Map of service name -> ServiceId (for lookup by name)
    name_to_id: BTreeMap<String, ServiceId>,

    /// Next ServiceId to allocate
    next_service_id: u64,
}

impl GroveManager {
    /// Create a new empty Grove Manager
    pub const fn new() -> Self {
        Self {
            services: BTreeMap::new(),
            name_to_id: BTreeMap::new(),
            next_service_id: 1,  // Start at 1, 0 is reserved
        }
    }

    /// Register a new Grove service
    ///
    /// # Arguments
    /// * `name` - Unique service name (e.g., "world-tree")
    /// * `vessel_id` - The Vessel this service runs in
    /// * `main_thread_id` - The main service thread
    /// * `capabilities` - Capabilities to grant to this service
    /// * `limits` - Resource limits for this service
    /// * `respawn_policy` - How to handle service crashes
    ///
    /// # Returns
    /// * `Ok(ServiceId)` - The ID of the newly registered service
    /// * `Err(GroveError)` - If registration fails
    pub fn register_service(
        &mut self,
        name: String,
        vessel_id: VesselId,
        main_thread_id: ThreadId,
        capabilities: Vec<ServiceCapability>,
        limits: ResourceLimits,
        respawn_policy: super::service::RespawnPolicy,
    ) -> Result<ServiceId, GroveError> {
        // Check if we've reached max services
        if self.services.len() >= MAX_SERVICES {
            return Err(GroveError::TooManyServices);
        }

        // Check if name is already taken
        if self.name_to_id.contains_key(&name) {
            return Err(GroveError::InvalidServiceName);
        }

        // Allocate new ServiceId
        let service_id = ServiceId(self.next_service_id);
        self.next_service_id += 1;

        // Create ServiceInfo
        let service_info = ServiceInfo::new(
            service_id,
            name.clone(),
            vessel_id,
            main_thread_id,
            capabilities,
            limits,
            respawn_policy,
        );

        // Insert into registry
        self.services.insert(service_id, service_info);
        self.name_to_id.insert(name, service_id);

        crate::serial_println!("[GroveManager] Registered service {} with ID {:?}",
                               self.services.get(&service_id).unwrap().name,
                               service_id);

        Ok(service_id)
    }

    /// Unregister a service (called when service terminates)
    pub fn unregister_service(&mut self, service_id: ServiceId) -> Result<(), GroveError> {
        let service_info = self.services.remove(&service_id)
            .ok_or(GroveError::ServiceNotFound)?;

        self.name_to_id.remove(&service_info.name);

        crate::serial_println!("[GroveManager] Unregistered service {} (ID {:?})",
                               service_info.name, service_id);

        Ok(())
    }

    /// Find a service by ID
    pub fn find_service(&self, service_id: ServiceId) -> Option<&ServiceInfo> {
        self.services.get(&service_id)
    }

    /// Find a service by ID (mutable)
    pub fn find_service_mut(&mut self, service_id: ServiceId) -> Option<&mut ServiceInfo> {
        self.services.get_mut(&service_id)
    }

    /// Find a service by name
    pub fn find_service_by_name(&self, name: &str) -> Option<&ServiceInfo> {
        self.name_to_id.get(name)
            .and_then(|id| self.services.get(id))
    }

    /// Get all registered services
    pub fn all_services(&self) -> impl Iterator<Item = &ServiceInfo> {
        self.services.values()
    }

    /// Mark a service as running
    pub fn set_service_running(&mut self, service_id: ServiceId) -> Result<(), GroveError> {
        let service = self.services.get_mut(&service_id)
            .ok_or(GroveError::ServiceNotFound)?;

        if service.state != ServiceState::Loading {
            return Err(GroveError::InvalidState);
        }

        service.set_running();
        crate::serial_println!("[GroveManager] Service {} is now running", service.name);

        Ok(())
    }

    /// Mark a service as faulted
    pub fn set_service_faulted(&mut self, service_id: ServiceId) -> Result<(), GroveError> {
        let service = self.services.get_mut(&service_id)
            .ok_or(GroveError::ServiceNotFound)?;

        service.set_faulted();
        crate::serial_println!("[GroveManager] Service {} has FAULTED", service.name);

        Ok(())
    }

    /// Mark a service as stopped
    pub fn set_service_stopped(&mut self, service_id: ServiceId) -> Result<(), GroveError> {
        let service = self.services.get_mut(&service_id)
            .ok_or(GroveError::ServiceNotFound)?;

        service.set_stopped();
        crate::serial_println!("[GroveManager] Service {} stopped gracefully", service.name);

        Ok(())
    }

    /// Check if a service has a specific capability
    pub fn check_capability(
        &self,
        service_id: ServiceId,
        capability: ServiceCapability,
    ) -> Result<bool, GroveError> {
        let service = self.services.get(&service_id)
            .ok_or(GroveError::ServiceNotFound)?;

        Ok(service.has_capability(capability))
    }

    /// Update resource usage for a service
    pub fn update_resource_usage(
        &mut self,
        service_id: ServiceId,
        cpu_time_used: u64,
        memory_used: usize,
        thread_count: usize,
    ) -> Result<(), GroveError> {
        let service = self.services.get_mut(&service_id)
            .ok_or(GroveError::ServiceNotFound)?;

        service.cpu_time_used = cpu_time_used;
        service.memory_used = memory_used;
        service.thread_count = thread_count;

        // Check if service has exceeded limits
        if !service.within_limits() {
            crate::serial_println!(
                "[GroveManager] WARNING: Service {} exceeded resource limits! \
                 Memory: {}/{}, Threads: {}/{}",
                service.name,
                service.memory_used,
                service.limits.max_memory,
                service.thread_count,
                service.limits.max_threads
            );
            return Err(GroveError::ResourceLimitExceeded);
        }

        Ok(())
    }

    /// Get the number of registered services
    pub fn service_count(&self) -> usize {
        self.services.len()
    }

    /// Get all services in a specific state
    pub fn services_in_state(&self, state: ServiceState) -> impl Iterator<Item = &ServiceInfo> {
        self.services.values()
            .filter(move |s| s.state == state)
    }
}

/// Global Grove Manager instance
static GROVE_MANAGER: Mutex<GroveManager> = Mutex::new(GroveManager::new());

/// Get a reference to the global Grove Manager
pub fn get_grove_manager() -> &'static Mutex<GroveManager> {
    &GROVE_MANAGER
}

/// Initialize the Grove Manager (called during kernel boot)
pub fn init() {
    crate::serial_println!("[GroveManager] Initialized - ready to manage Ring 1 services");
}
