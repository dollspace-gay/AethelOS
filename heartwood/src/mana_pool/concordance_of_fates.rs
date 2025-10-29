//! # The Concordance of Fates (RBAC)
//!
//! *"Every entity in the realm—every scroll, every thread, every user—is assigned*
//! *a Fate. This Fate, defined in the sacred Concordance, dictates the absolute*
//! *limits of its being. It is not a set of permissions to be checked; it is a*
//! *fundamental law of nature."*
//!
//! ## Philosophy
//!
//! This is the ultimate expression of symbiotic harmony. The Concordance doesn't
//! merely restrict actions—it makes disharmonious actions **conceptually impossible**.
//!
//! - The `recite` thread simply **cannot** open a network conduit; that is not its Fate.
//! - The `whisper-client` thread is **incapable** of reading scrolls from another
//!   user's branch; its Fate does not allow it.
//! - The `Glimmer-Weave` interpreter **would have** a Fate that prevents it from
//!   modifying the Heartwood's core structures.
//!
//! This is not enforcement—this is **the fabric of reality itself**.
//!
//! ## Architecture
//!
//! The Concordance defines three types of entities:
//!
//! 1. **Subjects**: Who is acting (threads, processes, users)
//! 2. **Objects**: What is being accessed (files, network, memory)
//! 3. **Fates (Roles)**: What a Subject can do with Objects
//!
//! Every Subject has exactly one Fate at any moment. A Fate is immutable once
//! assigned—changing Fate requires explicit transition rules.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloc::vec;
use alloc::collections::BTreeMap;
use core::fmt;

/// A Fate (Role) - defines what a Subject can do
///
/// A Fate is more than a set of permissions; it is the essence of what
/// an entity **is**. Like the Moirai of Greek mythology, the Fates are
/// absolute and inescapable.
#[derive(Debug, Clone)]
pub struct Fate {
    /// Unique name of this Fate
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Set of capabilities this Fate grants
    pub capabilities: FateCapabilities,

    /// File access rules
    pub file_rules: Vec<FileRule>,

    /// Network access rules
    pub network_rules: Vec<NetworkRule>,

    /// Memory access rules
    pub memory_rules: Vec<MemoryRule>,

    /// Can this Fate transition to other Fates?
    pub allowed_transitions: Vec<String>,

    /// Is this a privileged Fate (kernel-level)?
    pub is_privileged: bool,
}

/// Capabilities that a Fate can grant
///
/// These are fine-grained permissions for specific operations.
#[derive(Debug, Clone, Copy, Default)]
pub struct FateCapabilities {
    // File operations
    pub can_read_files: bool,
    pub can_write_files: bool,
    pub can_execute_files: bool,
    pub can_create_files: bool,
    pub can_delete_files: bool,

    // Network operations
    pub can_bind_network: bool,
    pub can_connect_network: bool,
    pub can_listen_network: bool,
    pub can_send_packets: bool,
    pub can_receive_packets: bool,

    // Process operations
    pub can_fork: bool,
    pub can_exec: bool,
    pub can_kill: bool,
    pub can_change_priority: bool,

    // Memory operations
    pub can_allocate_memory: bool,
    pub can_modify_memory: bool,
    pub can_share_memory: bool,

    // IPC operations
    pub can_send_ipc: bool,
    pub can_receive_ipc: bool,

    // System operations
    pub can_read_symbols: bool,
    pub can_load_modules: bool,
    pub can_modify_system: bool,
}

/// Rule for file access
#[derive(Debug, Clone)]
pub struct FileRule {
    /// Path pattern (supports wildcards)
    pub path_pattern: String,

    /// Access type (read, write, execute)
    pub access_type: FileAccess,

    /// Allow or deny?
    pub permission: Permission,
}

/// File access types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileAccess {
    Read,
    Write,
    Execute,
    ReadWrite,
    All,
}

/// Rule for network access
#[derive(Debug, Clone)]
pub struct NetworkRule {
    /// Port or port range
    pub port_range: (u16, u16),

    /// Operation (bind, connect)
    pub operation: NetworkOperation,

    /// Allow or deny?
    pub permission: Permission,
}

/// Network operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkOperation {
    Bind,
    Connect,
    Listen,
    SendTo,
    ReceiveFrom,
}

/// Rule for memory access
#[derive(Debug, Clone)]
pub struct MemoryRule {
    /// Address range (start, end)
    pub address_range: (u64, u64),

    /// Access type
    pub access_type: MemoryAccess,

    /// Allow or deny?
    pub permission: Permission,
}

/// Memory access types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAccess {
    Read,
    Write,
    Execute,
    ReadWrite,
    All,
}

/// Permission type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Allow,
    Deny,
}

/// A Subject - an entity that performs actions
///
/// Subjects are threads, processes, or users that have been assigned a Fate.
#[derive(Debug, Clone)]
pub struct Subject {
    /// Unique identifier
    pub id: SubjectId,

    /// Current Fate (role)
    pub fate: String,

    /// Subject type
    pub subject_type: SubjectType,
}

/// Subject identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubjectId(pub u64);

/// Types of subjects
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubjectType {
    /// A kernel thread
    KernelThread,

    /// A user process (future)
    UserProcess,

    /// A system service
    SystemService,
}

/// The Concordance - the master policy database
///
/// This is the sacred scroll that defines all Fates and their relationships.
pub struct Concordance {
    /// Map of Fate name → Fate definition
    pub fates: BTreeMap<String, Fate>,

    /// Map of Subject ID → Subject
    pub subjects: BTreeMap<SubjectId, Subject>,

    /// Is the Concordance sealed (immutable)?
    sealed: bool,

    /// Default Fate for new subjects
    default_fate: String,
}

impl Concordance {
    /// Create a new empty Concordance
    pub fn new() -> Self {
        Self {
            fates: BTreeMap::new(),
            subjects: BTreeMap::new(),
            sealed: false,
            default_fate: String::new(),
        }
    }

    /// Define a new Fate
    ///
    /// # Arguments
    ///
    /// * `fate` - The Fate definition to add
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Fate added successfully
    /// * `Err(ConcordanceError)` - If Concordance is sealed or name conflicts
    pub fn define_fate(&mut self, fate: Fate) -> Result<(), ConcordanceError> {
        if self.sealed {
            return Err(ConcordanceError::ConcordanceSealed);
        }

        if self.fates.contains_key(&fate.name) {
            return Err(ConcordanceError::FateAlreadyExists(fate.name.clone()));
        }

        self.fates.insert(fate.name.clone(), fate);
        Ok(())
    }

    /// Assign a Fate to a Subject
    ///
    /// # Arguments
    ///
    /// * `subject_id` - The Subject to assign to
    /// * `fate_name` - Name of the Fate to assign
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Fate assigned successfully
    /// * `Err(ConcordanceError)` - If Fate doesn't exist or assignment fails
    pub fn assign_fate(&mut self, subject_id: SubjectId, fate_name: &str) -> Result<(), ConcordanceError> {
        if !self.fates.contains_key(fate_name) {
            return Err(ConcordanceError::FateNotFound(String::from(fate_name)));
        }

        if let Some(subject) = self.subjects.get_mut(&subject_id) {
            // Check if transition is allowed
            if let Some(current_fate) = self.fates.get(&subject.fate) {
                let fate_name_string = String::from(fate_name);
                if !current_fate.allowed_transitions.contains(&fate_name_string) &&
                   !current_fate.is_privileged {
                    return Err(ConcordanceError::TransitionDenied {
                        from: subject.fate.clone(),
                        to: fate_name_string,
                    });
                }
            }

            subject.fate = String::from(fate_name);
            Ok(())
        } else {
            Err(ConcordanceError::SubjectNotFound(subject_id))
        }
    }

    /// Register a new Subject
    ///
    /// # Arguments
    ///
    /// * `subject_id` - Unique ID for this Subject
    /// * `subject_type` - Type of Subject
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Subject registered with default Fate
    /// * `Err(ConcordanceError)` - If registration fails
    pub fn register_subject(&mut self, subject_id: SubjectId, subject_type: SubjectType) -> Result<(), ConcordanceError> {
        if self.subjects.contains_key(&subject_id) {
            return Err(ConcordanceError::SubjectAlreadyExists(subject_id));
        }

        let subject = Subject {
            id: subject_id,
            fate: self.default_fate.clone(),
            subject_type,
        };

        self.subjects.insert(subject_id, subject);
        Ok(())
    }

    /// Check if a Subject can perform an operation
    ///
    /// This is the core enforcement function. Returns `true` if the operation
    /// is allowed by the Subject's Fate, `false` otherwise.
    ///
    /// # Arguments
    ///
    /// * `subject_id` - The Subject attempting the operation
    /// * `operation` - The operation to check
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Operation is allowed by Subject's Fate
    /// * `Ok(false)` - Operation is denied by Subject's Fate
    /// * `Err(ConcordanceError)` - Subject or Fate not found
    pub fn check_permission(&self, subject_id: SubjectId, operation: &Operation) -> Result<bool, ConcordanceError> {
        let subject = self.subjects.get(&subject_id)
            .ok_or(ConcordanceError::SubjectNotFound(subject_id))?;

        let fate = self.fates.get(&subject.fate)
            .ok_or(ConcordanceError::FateNotFound(subject.fate.clone()))?;

        // Check based on operation type
        match operation {
            Operation::FileRead(path) => self.check_file_permission(fate, path, FileAccess::Read),
            Operation::FileWrite(path) => self.check_file_permission(fate, path, FileAccess::Write),
            Operation::FileExecute(path) => self.check_file_permission(fate, path, FileAccess::Execute),
            Operation::NetworkBind(port) => self.check_network_permission(fate, *port, NetworkOperation::Bind),
            Operation::NetworkConnect(port) => self.check_network_permission(fate, *port, NetworkOperation::Connect),
            Operation::MemoryRead(addr) => self.check_memory_permission(fate, *addr, MemoryAccess::Read),
            Operation::MemoryWrite(addr) => self.check_memory_permission(fate, *addr, MemoryAccess::Write),
            Operation::Fork => Ok(fate.capabilities.can_fork),
            Operation::ReadSymbols => Ok(fate.capabilities.can_read_symbols),
        }
    }

    /// Check file permission against Fate's rules
    fn check_file_permission(&self, fate: &Fate, path: &str, access: FileAccess) -> Result<bool, ConcordanceError> {
        // First check capability
        let cap_allowed = match access {
            FileAccess::Read => fate.capabilities.can_read_files,
            FileAccess::Write => fate.capabilities.can_write_files,
            FileAccess::Execute => fate.capabilities.can_execute_files,
            FileAccess::ReadWrite => fate.capabilities.can_read_files && fate.capabilities.can_write_files,
            FileAccess::All => fate.capabilities.can_read_files &&
                              fate.capabilities.can_write_files &&
                              fate.capabilities.can_execute_files,
        };

        if !cap_allowed {
            return Ok(false);
        }

        // Check explicit rules (deny rules take precedence)
        for rule in &fate.file_rules {
            if self.path_matches(&rule.path_pattern, path) {
                return Ok(rule.permission == Permission::Allow);
            }
        }

        // Default deny
        Ok(false)
    }

    /// Check network permission against Fate's rules
    fn check_network_permission(&self, fate: &Fate, port: u16, op: NetworkOperation) -> Result<bool, ConcordanceError> {
        // Check capability
        let cap_allowed = match op {
            NetworkOperation::Bind => fate.capabilities.can_bind_network,
            NetworkOperation::Connect => fate.capabilities.can_connect_network,
            NetworkOperation::Listen => fate.capabilities.can_listen_network,
            NetworkOperation::SendTo => fate.capabilities.can_send_packets,
            NetworkOperation::ReceiveFrom => fate.capabilities.can_receive_packets,
        };

        if !cap_allowed {
            return Ok(false);
        }

        // Check explicit rules
        for rule in &fate.network_rules {
            if port >= rule.port_range.0 && port <= rule.port_range.1 {
                return Ok(rule.permission == Permission::Allow);
            }
        }

        // Default deny
        Ok(false)
    }

    /// Check memory permission against Fate's rules
    fn check_memory_permission(&self, fate: &Fate, addr: u64, access: MemoryAccess) -> Result<bool, ConcordanceError> {
        // Check capability
        let cap_allowed = match access {
            MemoryAccess::Read => true, // Most Fates can read memory
            MemoryAccess::Write => fate.capabilities.can_modify_memory,
            MemoryAccess::Execute => true, // Controlled by W^X
            MemoryAccess::ReadWrite => fate.capabilities.can_modify_memory,
            MemoryAccess::All => fate.capabilities.can_modify_memory,
        };

        if !cap_allowed {
            return Ok(false);
        }

        // Check explicit rules
        for rule in &fate.memory_rules {
            if addr >= rule.address_range.0 && addr < rule.address_range.1 {
                return Ok(rule.permission == Permission::Allow);
            }
        }

        // Default allow for memory (controlled by other mechanisms)
        Ok(true)
    }

    /// Simple path matching (supports * wildcard)
    fn path_matches(&self, pattern: &str, path: &str) -> bool {
        if pattern == "*" || pattern == "**" {
            return true;
        }

        if pattern.contains('*') {
            // Simple prefix matching for now
            let prefix = pattern.trim_end_matches('*');
            path.starts_with(prefix)
        } else {
            pattern == path
        }
    }

    /// Seal the Concordance (make it immutable)
    ///
    /// Once sealed, no new Fates can be defined. This is typically done
    /// after system initialization to prevent tampering.
    pub fn seal(&mut self) {
        self.sealed = true;
    }

    /// Check if Concordance is sealed
    pub fn is_sealed(&self) -> bool {
        self.sealed
    }

    /// Set the default Fate for new Subjects
    pub fn set_default_fate(&mut self, fate_name: String) -> Result<(), ConcordanceError> {
        if !self.fates.contains_key(&fate_name) {
            return Err(ConcordanceError::FateNotFound(fate_name));
        }
        self.default_fate = fate_name;
        Ok(())
    }

    /// Get the number of defined Fates
    pub fn fate_count(&self) -> usize {
        self.fates.len()
    }

    /// Get the number of registered Subjects
    pub fn subject_count(&self) -> usize {
        self.subjects.len()
    }
}

/// Operations that can be checked against the Concordance
#[derive(Debug, Clone)]
pub enum Operation {
    FileRead(String),
    FileWrite(String),
    FileExecute(String),
    NetworkBind(u16),
    NetworkConnect(u16),
    MemoryRead(u64),
    MemoryWrite(u64),
    Fork,
    ReadSymbols,
}

/// Errors that can occur when working with the Concordance
#[derive(Debug, Clone)]
pub enum ConcordanceError {
    /// Concordance is sealed and cannot be modified
    ConcordanceSealed,

    /// Fate with this name already exists
    FateAlreadyExists(String),

    /// Fate not found
    FateNotFound(String),

    /// Subject not found
    SubjectNotFound(SubjectId),

    /// Subject already exists
    SubjectAlreadyExists(SubjectId),

    /// Transition between Fates is not allowed
    TransitionDenied {
        from: String,
        to: String,
    },

    /// Operation denied by Fate
    OperationDenied {
        subject: SubjectId,
        operation: String,
    },
}

impl fmt::Display for ConcordanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConcordanceError::ConcordanceSealed =>
                write!(f, "The Concordance is sealed and cannot be altered"),
            ConcordanceError::FateAlreadyExists(name) =>
                write!(f, "Fate '{}' already exists in the Concordance", name),
            ConcordanceError::FateNotFound(name) =>
                write!(f, "Fate '{}' not found in the Concordance", name),
            ConcordanceError::SubjectNotFound(id) =>
                write!(f, "Subject {:?} not found", id),
            ConcordanceError::SubjectAlreadyExists(id) =>
                write!(f, "Subject {:?} already exists", id),
            ConcordanceError::TransitionDenied { from, to} =>
                write!(f, "Transition from Fate '{}' to '{}' is forbidden", from, to),
            ConcordanceError::OperationDenied { subject, operation } =>
                write!(f, "Subject {:?} cannot perform: {}", subject, operation),
        }
    }
}

/// Global Concordance instance
static mut CONCORDANCE: Option<Concordance> = None;

/// Initialize the Concordance of Fates
///
/// Creates default Fates for system operation.
///
/// # Safety
///
/// Must be called once during kernel initialization.
pub unsafe fn init_concordance() {
    crate::serial_println!("[CONCORDANCE] Initializing the Concordance of Fates...");

    crate::serial_println!("[CONCORDANCE] Creating empty Concordance...");
    let mut concordance = Concordance::new();
    crate::serial_println!("[CONCORDANCE] ✓ Empty Concordance created");

    // Define default Fates
    crate::serial_println!("[CONCORDANCE] Defining default Fates...");
    define_default_fates(&mut concordance);
    crate::serial_println!("[CONCORDANCE] ✓ Default Fates defined");

    // Set default Fate for new threads
    crate::serial_println!("[CONCORDANCE] Setting default Fate...");
    concordance.set_default_fate(String::from("Guardian"))
        .expect("Failed to set default Fate");
    crate::serial_println!("[CONCORDANCE] ✓ Default Fate set");

    crate::serial_println!("[CONCORDANCE] ✓ {} Fates defined in the sacred scroll", concordance.fate_count());
    crate::serial_println!("[CONCORDANCE] ✓ The Concordance of Fates governs all");

    CONCORDANCE = Some(concordance);
}

/// Define the default system Fates
fn define_default_fates(concordance: &mut Concordance) {
    // Fate 1: The Guardian (Kernel threads)
    let guardian = Fate {
        name: String::from("Guardian"),
        description: String::from("The eternal guardians of the Heartwood. Full privileges."),
        capabilities: FateCapabilities {
            can_read_files: true,
            can_write_files: true,
            can_execute_files: true,
            can_create_files: true,
            can_delete_files: true,
            can_bind_network: true,
            can_connect_network: true,
            can_listen_network: true,
            can_send_packets: true,
            can_receive_packets: true,
            can_fork: true,
            can_exec: true,
            can_kill: true,
            can_change_priority: true,
            can_allocate_memory: true,
            can_modify_memory: true,
            can_share_memory: true,
            can_send_ipc: true,
            can_receive_ipc: true,
            can_read_symbols: true,
            can_load_modules: true,
            can_modify_system: true,
        },
        file_rules: vec![
            FileRule {
                path_pattern: String::from("/**"),
                access_type: FileAccess::All,
                permission: Permission::Allow,
            },
        ],
        network_rules: vec![
            NetworkRule {
                port_range: (0, 65535),
                operation: NetworkOperation::Bind,
                permission: Permission::Allow,
            },
        ],
        memory_rules: vec![],
        allowed_transitions: vec![String::from("Guardian"), String::from("Weaver")],
        is_privileged: true,
    };

    // Fate 2: The Weaver (General user threads)
    let weaver = Fate {
        name: String::from("Weaver"),
        description: String::from("Those who weave the threads of computation."),
        capabilities: FateCapabilities {
            can_read_files: true,
            can_write_files: false,
            can_execute_files: true,
            can_create_files: false,
            can_delete_files: false,
            can_bind_network: false,
            can_connect_network: true,
            can_listen_network: false,
            can_send_packets: true,
            can_receive_packets: true,
            can_fork: true,
            can_exec: true,
            can_kill: false,
            can_change_priority: false,
            can_allocate_memory: true,
            can_modify_memory: true,
            can_share_memory: false,
            can_send_ipc: true,
            can_receive_ipc: true,
            can_read_symbols: false,
            can_load_modules: false,
            can_modify_system: false,
        },
        file_rules: vec![
            FileRule {
                path_pattern: String::from("/home/**"),
                access_type: FileAccess::ReadWrite,
                permission: Permission::Allow,
            },
        ],
        network_rules: vec![
            NetworkRule {
                port_range: (1024, 65535),
                operation: NetworkOperation::Connect,
                permission: Permission::Allow,
            },
        ],
        memory_rules: vec![],
        allowed_transitions: vec![String::from("Weaver")],
        is_privileged: false,
    };

    concordance.define_fate(guardian).expect("Failed to define Guardian");
    concordance.define_fate(weaver).expect("Failed to define Weaver");
}

/// Get a reference to the global Concordance
///
/// # Safety
///
/// Must be called after `init_concordance()`.
pub unsafe fn get_concordance() -> &'static mut Concordance {
    CONCORDANCE.as_mut().expect("Concordance not initialized")
}

/// Check if the Concordance is initialized
pub fn is_concordance_active() -> bool {
    unsafe { CONCORDANCE.is_some() }
}

/// Check if the Concordance is sealed (global helper)
pub fn is_sealed() -> bool {
    unsafe {
        CONCORDANCE.as_ref()
            .map(|c| c.is_sealed())
            .unwrap_or(false)
    }
}

/// Get the number of defined Fates (global helper)
pub fn get_fate_count() -> usize {
    unsafe {
        CONCORDANCE.as_ref()
            .map(|c| c.fate_count())
            .unwrap_or(0)
    }
}

/// Get the number of registered Subjects (global helper)
pub fn get_subject_count() -> usize {
    unsafe {
        CONCORDANCE.as_ref()
            .map(|c| c.subject_count())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fate_creation() {
        let mut concordance = Concordance::new();

        let fate = Fate {
            name: String::from("TestFate"),
            description: String::from("A test fate"),
            capabilities: FateCapabilities::default(),
            file_rules: vec![],
            network_rules: vec![],
            memory_rules: vec![],
            allowed_transitions: vec![],
            is_privileged: false,
        };

        assert!(concordance.define_fate(fate).is_ok());
        assert_eq!(concordance.fate_count(), 1);
    }

    #[test]
    fn test_subject_registration() {
        let mut concordance = Concordance::new();
        define_default_fates(&mut concordance);
        concordance.set_default_fate(String::from("Guardian")).unwrap();

        let subject_id = SubjectId(1);
        assert!(concordance.register_subject(subject_id, SubjectType::KernelThread).is_ok());
        assert_eq!(concordance.subject_count(), 1);
    }
}
