# World-Tree Grove Implementation Plan

**Status:** Design Phase
**Created:** January 2025
**Architecture:** Git-like Content-Addressable Storage with Query-Based Access

---

## Overview

The World-Tree Grove is AethelOS's radical reimagining of filesystem storage. Instead of hierarchical paths, files are discovered through queries. Instead of manual versioning, all changes are tracked automatically. Instead of blind file typing, the system has "true sight" to detect content type.

### Core Design Decisions

✅ **Git-like architecture**: Content-addressable storage with SHA-256 hashing
✅ **String-based queries**: Flexible query language (not Rust structs)
✅ **Global version tracking**: Like git commits, with per-file rollback capability
✅ **Extensible Essences**: Apps can define custom file types
✅ **Auto-detect Essence**: Content-based detection regardless of filename
✅ **Delegatable capabilities**: Processes can share subsets of rights
✅ **RAM disk for initial testing**: No hardware drivers needed initially

---

## Architecture Components

### 1. Content-Addressable Object Store

Files are stored by their SHA-256 hash, enabling:
- Natural deduplication (same content = same hash)
- Delta storage for versions
- Content integrity verification
- Efficient storage of similar files

```
objects/
  ab/
    cd1234567890abcdef...  (file content)
  12/
    34567890abcdef1234...  (file content)
```

### 2. Metadata Index

Metadata is stored separately from content, allowing:
- Fast queries without reading file content
- Multiple "names" pointing to same content
- Rich attribute indexing

```rust
struct Metadata {
    id: FileId,                    // Unique ID for this version
    name: String,                  // Human-readable name
    essence: Essence,              // Auto-detected type
    creator: UserId,               // Who created it
    created: Timestamp,            // When created
    modified: Timestamp,           // Last modification
    content_hash: SHA256Hash,      // Points to object store
    parent: Option<FileId>,        // Version chain
    tags: HashMap<String, String>, // Custom metadata
    size: usize,                   // Content size in bytes
}
```

### 3. Global Commit Graph

Like git, all changes are tracked in commits:

```rust
struct Commit {
    id: CommitId,
    timestamp: Timestamp,
    author: UserId,
    message: String,
    changes: Vec<Change>,          // What changed
    parent: Option<CommitId>,      // Previous commit
}

enum Change {
    Create { file: FileId, content_hash: SHA256Hash },
    Modify { file: FileId, old_hash: SHA256Hash, new_hash: SHA256Hash },
    Delete { file: FileId },
}
```

### 4. Query Index

Multi-dimensional indexing for fast queries:

```rust
struct QueryIndex {
    by_name: BTreeMap<String, Vec<FileId>>,
    by_essence: HashMap<Essence, Vec<FileId>>,
    by_creator: HashMap<UserId, Vec<FileId>>,
    by_timestamp: BTreeMap<Timestamp, Vec<FileId>>,
    by_tag: HashMap<(String, String), Vec<FileId>>,
    by_content_hash: HashMap<SHA256Hash, Vec<FileId>>,
}
```

---

## Query Language

### Syntax

String-based queries with logical operators:

```
# Simple field queries
essence:Scroll
creator:Elara
name:config

# Temporal queries
created:>2025-01-15
modified:<2025-01-20
created:2025-01-15..2025-01-20

# Tag queries
tag:project=AethelOS
tag:language=Rust

# Logical operators
essence:Scroll AND creator:Elara
(essence:Scroll OR essence:Tome) AND created:last_week
essence:Rune AND NOT tag:archived=true

# Future: Content search
contains:"Symbiotic Computing"
regex:"fn\s+\w+\("
```

### Parser Pipeline

```rust
// 1. Tokenize
let tokens = tokenize(query_string)?;
// ["essence", ":", "Scroll", "AND", "creator", ":", "Elara"]

// 2. Parse into AST
let ast = parse(tokens)?;
// And(
//     Field("essence", Equals, "Scroll"),
//     Field("creator", Equals, "Elara")
// )

// 3. Optimize query plan
let plan = optimize(ast, &index)?;
// Use index.by_essence first (smaller result set),
// then filter by creator

// 4. Execute
let results = execute(plan, &index)?;
// Vec<FileId>
```

---

## Essence Auto-Detection

The World-Tree has "true sight" - it detects file type from content, not names or extensions.

### Detection Pipeline

```rust
fn detect_essence(content: &[u8]) -> Essence {
    // 1. Magic numbers (first bytes)
    match &content[..min(content.len(), 16)] {
        [0x7f, b'E', b'L', b'F', ..] => return Essence::Tome,
        [0x89, b'P', b'N', b'G', ..] => return Essence::Image(ImageType::PNG),
        [0xff, 0xd8, 0xff, ..] => return Essence::Image(ImageType::JPEG),
        _ => {}
    }

    // 2. UTF-8 text analysis
    if let Ok(text) = str::from_utf8(content) {
        if looks_like_code(text) {
            return detect_code_language(text);
        }
        if looks_like_config(text) {
            return Essence::Rune;
        }
        if looks_like_markup(text) {
            return detect_markup_type(text);
        }
        return Essence::Scroll(ScrollType::PlainText);
    }

    // 3. Custom detectors (extensible)
    for detector in &REGISTERED_DETECTORS {
        if let Some(essence) = detector.detect(content) {
            return essence;
        }
    }

    // 4. Unknown
    Essence::Unknown
}
```

### Extensible Essence Registry

Applications can register custom detectors:

```rust
struct EssenceDetector {
    name: String,
    description: String,
    detect: fn(&[u8]) -> Option<Essence>,
    priority: u32,  // Higher priority runs first
}

// Example: Register custom format detector
world_tree.register_essence_detector(EssenceDetector {
    name: "MyCustomFormat".to_string(),
    description: "Detects .mcf files".to_string(),
    detect: |content| {
        if content.starts_with(b"MCF\x01") {
            Some(Essence::Custom("MyCustomFormat"))
        } else {
            None
        }
    },
    priority: 100,
});
```

### Core Essences

```rust
pub enum Essence {
    Scroll(ScrollType),    // Text documents
    Tome,                  // Executables
    Rune,                  // Configuration files
    Image(ImageType),      // Graphics
    Sound(SoundType),      // Audio
    Weave,                 // UI definitions
    Chronicle,             // Logs
    Sanctuary,             // Collections/directories
    Whisper,               // Temporary data
    Memory,                // Memory dumps
    Portal,                // Links to other files
    Custom(String),        // User-defined
    Unknown,               // Could not detect
}

pub enum ScrollType {
    PlainText,
    Code(Language),
    Markup(MarkupType),
    Poem,
}

pub enum Language {
    Rust,
    C,
    Python,
    JavaScript,
    // ...
}
```

---

## Versioning & Rollback

### Global Commit Chain

Every change creates a commit in the global chain:

```rust
impl WorldTree {
    pub fn commit(&mut self, files: Vec<(FileId, Vec<u8>)>, message: &str) -> CommitId {
        let mut changes = Vec::new();

        for (file_id, content) in files {
            // Hash the content
            let new_hash = sha256(&content);

            // Store in object store
            self.objects.insert(new_hash, Arc::new(content));

            // Detect essence
            let essence = detect_essence(&content);

            // Update metadata
            let old_metadata = self.metadata.get(&file_id);
            let new_metadata = Metadata {
                id: file_id,
                content_hash: new_hash,
                essence,
                modified: now(),
                parent: Some(file_id),
                ..old_metadata.clone()
            };

            self.metadata.insert(file_id, new_metadata);

            // Record change
            changes.push(Change::Modify {
                file: file_id,
                old_hash: old_metadata.content_hash,
                new_hash,
            });
        }

        // Create commit
        let commit = Commit {
            id: CommitId::new(),
            timestamp: now(),
            author: current_user(),
            message: message.to_string(),
            changes,
            parent: self.head,
        };

        self.commits.push(commit.clone());
        self.head = Some(commit.id);

        commit.id
    }
}
```

### Per-File Rollback

Despite global commits, you can rollback individual files:

```rust
impl WorldTree {
    pub fn rollback_file(
        &mut self,
        file: FileId,
        to_commit: CommitId,
        capability: Capability
    ) -> Result<FileId> {
        // Check permissions
        verify_capability(&capability, file, Rights::WRITE)?;

        // Find file version at target commit
        let old_hash = self.find_file_at_commit(file, to_commit)?;

        // Get old content (already in object store)
        let old_content = self.objects.get(&old_hash)
            .ok_or("Content not found")?;

        // Create new version pointing to old content
        let new_metadata = Metadata {
            id: FileId::new(),
            name: format!("{} (rolled back)", self.metadata[&file].name),
            content_hash: old_hash,  // Reuse old content!
            parent: Some(file),
            created: now(),
            modified: now(),
            ..self.metadata[&file].clone()
        };

        self.metadata.insert(new_metadata.id, new_metadata.clone());

        // Create rollback commit
        self.commit(
            vec![(new_metadata.id, (&**old_content).clone())],
            &format!("Rollback {} to commit {}", file, to_commit)
        );

        Ok(new_metadata.id)
    }

    fn find_file_at_commit(&self, file: FileId, commit: CommitId) -> Result<SHA256Hash> {
        // Walk back through commits until we find this file
        let mut current = Some(commit);

        while let Some(commit_id) = current {
            let commit = &self.commits[commit_id];

            // Check if this commit modified our file
            for change in &commit.changes {
                match change {
                    Change::Modify { file: f, new_hash, .. } if *f == file => {
                        return Ok(*new_hash);
                    }
                    Change::Create { file: f, content_hash } if *f == file => {
                        return Ok(*content_hash);
                    }
                    _ => {}
                }
            }

            current = commit.parent;
        }

        Err("File not found at commit")
    }
}
```

---

## Capability System

### Capability Structure

```rust
pub struct Capability {
    token: u128,              // Unforgeable random token
    object: FileId,           // What file this grants access to
    rights: Rights,           // What operations are allowed
    granted_by: UserId,       // Who created this capability
    granted_to: UserId,       // Who holds this capability
    created: Timestamp,       // When granted
    expires: Option<Timestamp>, // Optional expiration
}

bitflags! {
    pub struct Rights: u32 {
        const READ    = 0b00001;  // Can read content
        const WRITE   = 0b00010;  // Can modify content
        const EXECUTE = 0b00100;  // Can execute (if Tome)
        const DELETE  = 0b01000;  // Can delete file
        const SHARE   = 0b10000;  // Can delegate capabilities
    }
}
```

### Operations Requiring Capabilities

```rust
impl WorldTree {
    pub fn read(&self, file: FileId, cap: &Capability) -> Result<Arc<Vec<u8>>> {
        verify_capability(cap, file, Rights::READ)?;

        let metadata = self.metadata.get(&file)
            .ok_or("File not found")?;

        let content = self.objects.get(&metadata.content_hash)
            .ok_or("Content not found")?;

        Ok(content.clone())
    }

    pub fn write(&mut self, file: FileId, content: Vec<u8>, cap: &Capability) -> Result<()> {
        verify_capability(cap, file, Rights::WRITE)?;

        self.commit(vec![(file, content)], "Update file");
        Ok(())
    }

    pub fn delete(&mut self, file: FileId, cap: &Capability) -> Result<()> {
        verify_capability(cap, file, Rights::DELETE)?;

        // Soft delete: create tombstone commit
        let commit = Commit {
            id: CommitId::new(),
            timestamp: now(),
            author: current_user(),
            message: format!("Delete {}", file),
            changes: vec![Change::Delete { file }],
            parent: self.head,
        };

        self.commits.push(commit);
        Ok(())
    }
}

fn verify_capability(cap: &Capability, file: FileId, required: Rights) -> Result<()> {
    // Check token is valid
    if !CAPABILITY_STORE.contains(&cap.token) {
        return Err("Invalid capability token");
    }

    // Check it's for the right file
    if cap.object != file {
        return Err("Capability not for this file");
    }

    // Check rights
    if !cap.rights.contains(required) {
        return Err("Insufficient rights");
    }

    // Check expiration
    if let Some(expires) = cap.expires {
        if now() > expires {
            return Err("Capability expired");
        }
    }

    Ok(())
}
```

### Delegation (Subset of Rights)

```rust
impl WorldTree {
    pub fn delegate_capability(
        &mut self,
        original: &Capability,
        to_user: UserId,
        new_rights: Rights
    ) -> Result<Capability> {
        // Can only delegate if you have SHARE right
        if !original.rights.contains(Rights::SHARE) {
            return Err("No permission to share");
        }

        // Can only delegate rights you have
        if !original.rights.contains(new_rights) {
            return Err("Cannot delegate rights you don't have");
        }

        // Create new capability with subset of rights
        let delegated = Capability {
            token: random(),
            object: original.object,
            rights: new_rights,  // Subset!
            granted_by: current_user(),
            granted_to: to_user,
            created: now(),
            expires: original.expires,  // Inherit expiration
        };

        // Store in capability table
        CAPABILITY_STORE.insert(delegated.token, delegated.clone());

        Ok(delegated)
    }
}
```

**Example delegation scenario:**
```rust
// Alice has full access to a file
let alice_cap = Capability {
    rights: Rights::READ | Rights::WRITE | Rights::SHARE,
    // ...
};

// Alice delegates read-only access to Bob
let bob_cap = world_tree.delegate_capability(
    &alice_cap,
    UserId::Bob,
    Rights::READ  // Bob can only read
)?;

// Bob can read the file
let content = world_tree.read(file_id, &bob_cap)?;

// Bob cannot modify (this will fail)
world_tree.write(file_id, new_content, &bob_cap)?;  // Error!

// Bob cannot delegate to Charlie (no SHARE right)
world_tree.delegate_capability(&bob_cap, UserId::Charlie, Rights::READ)?;  // Error!
```

---

## Discovery Mechanism

**Recommendation: Public Metadata, Private Content**

This balances discoverability with security:
- Anyone can query and see metadata (Name, Essence, Creator, Timestamp)
- But you need a capability to read/modify actual content
- Like a library catalog - you can see what exists, but need permission to check out

```rust
impl WorldTree {
    // Query is public - anyone can search metadata
    pub fn query(&self, query_str: &str) -> Vec<FileMetadata> {
        let query = parse_query(query_str)?;
        let file_ids = execute_query(&query, &self.index)?;

        // Return metadata (not content!)
        file_ids.iter()
            .filter_map(|id| self.metadata.get(id))
            .map(|m| FileMetadata {
                id: m.id,
                name: m.name.clone(),
                essence: m.essence,
                creator: m.creator,
                created: m.created,
                size: m.size,
                // NO content!
            })
            .collect()
    }

    // But reading content requires capability
    pub fn read(&self, file: FileId, cap: &Capability) -> Result<Arc<Vec<u8>>> {
        verify_capability(cap, file, Rights::READ)?;
        // ...
    }
}
```

**Future enhancement: Visibility controls**
```rust
struct Metadata {
    // ...
    visibility: Visibility,
}

enum Visibility {
    Public,           // Appears in all queries
    Private,          // Only visible to creator
    Shared(Vec<UserId>), // Only visible to specific users
}
```

---

## RAM Disk Implementation

### Data Structure

```rust
pub struct RamDisk {
    // Content-addressable object store
    objects: HashMap<SHA256Hash, Arc<Vec<u8>>>,

    // File metadata
    metadata: HashMap<FileId, Metadata>,

    // Global commit chain
    commits: Vec<Commit>,
    head: Option<CommitId>,

    // Query index
    index: QueryIndex,

    // Capability store
    capabilities: HashMap<u128, Capability>,  // token -> capability

    // Auto-generated IDs
    next_file_id: AtomicU64,
    next_commit_id: AtomicU64,
}
```

### Core Operations

```rust
impl RamDisk {
    pub fn new() -> Self {
        RamDisk {
            objects: HashMap::new(),
            metadata: HashMap::new(),
            commits: Vec::new(),
            head: None,
            index: QueryIndex::new(),
            capabilities: HashMap::new(),
            next_file_id: AtomicU64::new(1),
            next_commit_id: AtomicU64::new(1),
        }
    }

    pub fn create(&mut self, name: &str, content: Vec<u8>) -> FileId {
        // Hash content
        let hash = sha256(&content);

        // Store in object store
        self.objects.insert(hash, Arc::new(content.clone()));

        // Detect essence
        let essence = detect_essence(&content);

        // Create metadata
        let id = FileId(self.next_file_id.fetch_add(1, Ordering::SeqCst));
        let metadata = Metadata {
            id,
            name: name.to_string(),
            essence,
            creator: current_user(),
            created: now(),
            modified: now(),
            content_hash: hash,
            parent: None,
            tags: HashMap::new(),
            size: content.len(),
        };

        // Store metadata
        self.metadata.insert(id, metadata.clone());

        // Update index
        self.index.add(&metadata);

        // Create commit
        self.commit(vec![(id, content)], &format!("Create {}", name));

        id
    }

    pub fn query(&self, query_str: &str) -> Result<Vec<FileMetadata>> {
        let ast = parse_query(query_str)?;
        let file_ids = execute_query(&ast, &self.index)?;

        Ok(file_ids.iter()
            .filter_map(|id| self.metadata.get(id))
            .map(|m| FileMetadata::from(m))
            .collect())
    }

    pub fn read(&self, file: FileId, cap: &Capability) -> Result<Arc<Vec<u8>>> {
        verify_capability(cap, file, Rights::READ)?;

        let metadata = self.metadata.get(&file)
            .ok_or("File not found")?;

        self.objects.get(&metadata.content_hash)
            .cloned()
            .ok_or("Content not found")
    }

    pub fn write(&mut self, file: FileId, content: Vec<u8>, cap: &Capability) -> Result<()> {
        verify_capability(cap, file, Rights::WRITE)?;

        let hash = sha256(&content);
        self.objects.insert(hash, Arc::new(content.clone()));

        // Update metadata
        if let Some(metadata) = self.metadata.get_mut(&file) {
            metadata.content_hash = hash;
            metadata.modified = now();
            metadata.size = content.len();
            metadata.essence = detect_essence(&content);
        }

        self.commit(vec![(file, content)], "Update file");
        Ok(())
    }
}
```

---

## Pruning & Space Management: "Dead Wood Removal"

### Philosophy

> *"Even the oldest tree must shed its dead wood.
> We honor the past, but we do not let it strangle the present.
> The World-Tree remembers, but it also forgets - wisely, gently, deliberately."*

**The Challenge:**
Git-like versioning creates a fundamental tension:
- **Users want:** Complete history, instant rollback, data safety
- **Reality demands:** Finite storage, reasonable performance, manageable complexity

**The Solution:**
The World-Tree is an **archival system**, not a hard drive eater. It balances preservation with pragmatism through intelligent pruning.

### Core Principles

1. **Current versions are sacred** - HEAD commit is never pruned
2. **Recent history is valuable** - Last N days kept in full
3. **Ancient history is compressed** - Old versions use delta storage
4. **User has final say** - Manual anchors preserve critical versions
5. **Pruning is explicit** - Never auto-delete without user awareness

---

### Retention Policies

```rust
pub struct RetentionPolicy {
    /// Keep all versions from last N days (full objects)
    keep_recent_days: u32,

    /// Keep weekly snapshots for N months
    keep_weekly_months: u32,

    /// Keep monthly snapshots for N years
    keep_monthly_years: u32,

    /// Keep user-anchored versions forever
    keep_anchors_forever: bool,

    /// Use delta compression for old versions
    use_delta_compression: bool,
}

// Default: Balanced retention (recommended)
pub const DEFAULT_RETENTION: RetentionPolicy = RetentionPolicy {
    keep_recent_days: 30,        // Full history for 1 month
    keep_weekly_months: 6,       // Weekly snapshots for 6 months
    keep_monthly_years: 2,       // Monthly snapshots for 2 years
    keep_anchors_forever: true,
    use_delta_compression: true,
};

// Archival: Maximum preservation
pub const ARCHIVAL_RETENTION: RetentionPolicy = RetentionPolicy {
    keep_recent_days: 365,       // 1 year full history
    keep_weekly_months: 24,      // 2 years of weekly snapshots
    keep_monthly_years: 10,      // 10 years of monthly snapshots
    keep_anchors_forever: true,
    use_delta_compression: true,
};

// Minimal: Low storage environments
pub const MINIMAL_RETENTION: RetentionPolicy = RetentionPolicy {
    keep_recent_days: 7,         // 1 week full history
    keep_weekly_months: 1,       // 4 weekly snapshots
    keep_monthly_years: 0,       // No long-term monthly snapshots
    keep_anchors_forever: true,
    use_delta_compression: true,
};
```

**Timeline Example (Default Policy):**

```
Today ─────────────────────────────────────────────────────────────► Past

├─ 0-30 days ─┤─ 1-6 months ─┤─ 6mo-2yr ─┤─ 2+ years ─┤
   EVERY         WEEKLY         MONTHLY      ANCHORS
   VERSION       SNAPSHOTS      SNAPSHOTS    ONLY

Storage:      High            Medium         Low          Minimal
Granularity:  Maximum         Weekly         Monthly      Critical only
```

---

### Pruning Strategies

#### Strategy 1: Reference-Counted Garbage Collection

Like `git gc`, removes unreachable objects:

```rust
pub struct Pruner {
    policy: RetentionPolicy,
    world_tree: Arc<Mutex<WorldTree>>,
}

impl Pruner {
    /// Mark-and-sweep garbage collection
    pub fn prune_unreachable(&mut self) -> PruneStats {
        // Phase 1: Mark all reachable objects
        let mut reachable = HashSet::new();
        self.mark_reachable(self.world_tree.head, &mut reachable);

        // Phase 2: Sweep unreachable objects
        let mut pruned_objects = 0;
        let mut space_reclaimed = 0;

        self.world_tree.objects.retain(|hash, content| {
            if reachable.contains(hash) {
                true
            } else {
                pruned_objects += 1;
                space_reclaimed += content.len();
                false
            }
        });

        PruneStats {
            objects_removed: pruned_objects,
            commits_removed: 0,
            space_reclaimed,
        }
    }

    fn mark_reachable(&self, commit_id: Option<CommitId>, reachable: &mut HashSet<SHA256Hash>) {
        let mut current = commit_id;

        while let Some(id) = current {
            let commit = &self.world_tree.commits[id];

            // Mark all objects referenced by this commit
            for change in &commit.changes {
                match change {
                    Change::Create { content_hash, .. } => {
                        reachable.insert(*content_hash);
                    }
                    Change::Modify { new_hash, .. } => {
                        reachable.insert(*new_hash);
                    }
                    _ => {}
                }
            }

            current = commit.parent;
        }
    }
}
```

#### Strategy 2: Time-Based Commit Thinning

Removes non-snapshot commits based on retention policy:

```rust
impl Pruner {
    /// Prune commits according to retention policy
    pub fn prune_by_policy(&mut self) -> PruneStats {
        let now = now();
        let mut commits_to_remove = Vec::new();

        for commit in &self.world_tree.commits {
            // Never prune HEAD
            if Some(commit.id) == self.world_tree.head {
                continue;
            }

            // Check if any files in this commit are anchored
            if self.has_anchored_files(commit) {
                continue;
            }

            let age_days = (now - commit.timestamp).days();

            // Keep recent commits (last N days)
            if age_days < self.policy.keep_recent_days {
                continue;
            }

            // Keep weekly snapshots (first commit of each week)
            if age_days < self.policy.keep_weekly_months * 30 {
                if commit.timestamp.is_start_of_week() {
                    continue;
                }
            }

            // Keep monthly snapshots (first commit of each month)
            else if age_days < self.policy.keep_monthly_years * 365 {
                if commit.timestamp.is_start_of_month() {
                    continue;
                }
            }

            // This commit can be pruned
            commits_to_remove.push(commit.id);
        }

        // Remove commits and run GC to clean up orphaned objects
        self.remove_commits(&commits_to_remove);
        self.prune_unreachable()
    }

    fn has_anchored_files(&self, commit: &Commit) -> bool {
        for change in &commit.changes {
            let file_id = match change {
                Change::Create { file, .. } => *file,
                Change::Modify { file, .. } => *file,
                Change::Delete { file } => *file,
            };

            if let Some(metadata) = self.world_tree.metadata.get(&file_id) {
                if metadata.anchored {
                    return true;
                }
            }
        }
        false
    }
}
```

#### Strategy 3: Delta Compression

Store old versions as diffs instead of full copies:

```rust
pub enum ObjectStorage {
    /// Full object (recent versions)
    Full(Arc<Vec<u8>>),

    /// Delta from base object (old versions)
    Delta {
        base_hash: SHA256Hash,
        diff: Vec<u8>,  // xdelta3 or similar
    },
}

impl WorldTree {
    /// Convert full object to delta compression
    pub fn compress_old_versions(&mut self) -> CompressionStats {
        let cutoff = now() - Duration::days(self.policy.keep_recent_days);
        let mut compressed = 0;
        let mut space_saved = 0;

        for commit in &self.commits {
            if commit.timestamp > cutoff {
                continue;  // Keep recent versions as full objects
            }

            for change in &commit.changes {
                if let Change::Modify { old_hash, new_hash, .. } = change {
                    // Convert new_hash to delta from old_hash
                    if let Some(space) = self.convert_to_delta(*old_hash, *new_hash) {
                        compressed += 1;
                        space_saved += space;
                    }
                }
            }
        }

        CompressionStats { compressed, space_saved }
    }

    fn convert_to_delta(&mut self, base: SHA256Hash, target: SHA256Hash) -> Option<usize> {
        let base_content = self.objects.get(&base)?;
        let target_content = self.objects.get(&target)?;

        // Already a delta
        if matches!(target_content, ObjectStorage::Delta { .. }) {
            return None;
        }

        // Compute diff
        let diff = compute_diff(base_content, target_content);
        let original_size = target_content.len();
        let delta_size = diff.len();

        // Only compress if we save significant space (>20%)
        if delta_size < original_size * 80 / 100 {
            self.objects.insert(target, ObjectStorage::Delta {
                base_hash: base,
                diff,
            });
            Some(original_size - delta_size)
        } else {
            None
        }
    }
}
```

#### Strategy 4: User-Controlled Anchors

Let users preserve critical versions forever:

```rust
pub struct Metadata {
    // ... existing fields ...

    /// If true, this version will NEVER be pruned
    pub anchored: bool,

    /// Optional reason for anchoring
    pub anchor_reason: Option<String>,

    /// Who anchored this version
    pub anchored_by: Option<UserId>,

    /// When it was anchored
    pub anchored_at: Option<Timestamp>,
}

impl WorldTree {
    /// Anchor a specific file version to prevent pruning
    pub fn anchor_version(
        &mut self,
        file: FileId,
        reason: &str,
        capability: &Capability
    ) -> Result<()> {
        verify_capability(capability, file, Rights::WRITE)?;

        if let Some(metadata) = self.metadata.get_mut(&file) {
            metadata.anchored = true;
            metadata.anchor_reason = Some(reason.to_string());
            metadata.anchored_by = Some(current_user());
            metadata.anchored_at = Some(now());
        }

        Ok(())
    }

    /// Remove anchor (allow pruning)
    pub fn unanchor_version(
        &mut self,
        file: FileId,
        capability: &Capability
    ) -> Result<()> {
        verify_capability(capability, file, Rights::WRITE)?;

        if let Some(metadata) = self.metadata.get_mut(&file) {
            metadata.anchored = false;
            metadata.anchor_reason = None;
        }

        Ok(())
    }

    /// List all anchored versions
    pub fn list_anchors(&self) -> Vec<AnchorInfo> {
        self.metadata.values()
            .filter(|m| m.anchored)
            .map(|m| AnchorInfo {
                file_id: m.id,
                name: m.name.clone(),
                reason: m.anchor_reason.clone(),
                anchored_by: m.anchored_by,
                anchored_at: m.anchored_at,
            })
            .collect()
    }
}
```

---

### Shell Commands

```bash
# View pruning statistics
eldarin> prune-stats
◈ Dead Wood Analysis
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Total commits:        3,842
  Reachable commits:    1,245 (32.4%)
  Prunable commits:     2,597 (67.6%)

  Total objects:        45,823
  Reachable objects:    12,441 (27.1%)
  Unreachable objects:  33,382 (72.9%)

  Storage usage:        8.7 GB
  Reclaimable space:    4.2 GB (48.3%)

# Dry-run prune (preview what would be deleted)
eldarin> prune --dry-run
◈ Pruning Simulation
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Would remove:
  • 2,597 commits (older than retention policy)
  • 33,382 objects (unreachable)

Retention breakdown:
  • Last 30 days:        1,245 commits (kept)
  • Weekly snapshots:    124 commits (kept)
  • Monthly snapshots:   24 commits (kept)
  • Anchored versions:   15 commits (kept)
  • Prunable:            2,597 commits (removed)

Space to reclaim: 4.2 GB

# Execute pruning
eldarin> prune --confirm
◈ Pruning dead wood...
  Removing old commits... 2,597 removed
  Garbage collecting... 33,382 objects freed
  Reclaimed 4.2 GB in 2.3s
✓ Pruning complete

# Anchor a version (preserve forever)
eldarin> anchor <file-id> "Production release v1.0.0"
✓ Version anchored: config.toml @ commit abc123

# List anchored versions
eldarin> list-anchors
◈ Anchored Versions
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  config.toml @ abc123
    "Working production config"
    Anchored by: root on 2025-01-15

  kernel.bin @ def456
    "v1.0 release - stable"
    Anchored by: root on 2025-01-01

  database.db @ 789abc
    "Pre-migration snapshot"
    Anchored by: admin on 2024-12-20

# Remove anchor (allow pruning)
eldarin> unanchor <file-id>
✓ Anchor removed from config.toml

# Configure retention policy
eldarin> retention-policy --set archival
✓ Retention policy set to 'archival'
  Keep recent: 365 days
  Weekly snapshots: 24 months
  Monthly snapshots: 10 years

eldarin> retention-policy --show
Current policy: default
  • Last 30 days: Full history
  • 1-6 months: Weekly snapshots
  • 6mo-2yr: Monthly snapshots
  • 2+ years: Anchored versions only
  • Delta compression: Enabled
```

---

### Safety Mechanisms

#### 1. Two-Phase Pruning

Never delete immediately - give user a chance to review:

```rust
pub struct PruneOperation {
    id: OperationId,
    created_at: Timestamp,
    commits_marked: Vec<CommitId>,
    objects_marked: Vec<SHA256Hash>,
    estimated_space: usize,
    executed: bool,
}

impl Pruner {
    /// Phase 1: Mark objects for deletion (reversible)
    pub fn mark_for_pruning(&mut self) -> OperationId {
        let op = self.prune_by_policy();

        // Don't actually delete yet, just record what would be deleted
        let operation = PruneOperation {
            id: OperationId::new(),
            created_at: now(),
            commits_marked: op.commits_to_remove,
            objects_marked: op.objects_to_remove,
            estimated_space: op.space_reclaimed,
            executed: false,
        };

        self.pending_operations.push(operation.clone());
        operation.id
    }

    /// Phase 2: Execute marked pruning (irreversible)
    pub fn execute_pruning(&mut self, op_id: OperationId) -> Result<PruneStats> {
        let op = self.pending_operations.iter_mut()
            .find(|o| o.id == op_id)
            .ok_or("Operation not found")?;

        if op.executed {
            return Err("Already executed");
        }

        // Actually delete
        let stats = self.do_prune(&op.commits_marked, &op.objects_marked);
        op.executed = true;

        Ok(stats)
    }
}
```

#### 2. Prune Audit Log

Record every pruning operation for accountability:

```rust
pub struct PruneLog {
    timestamp: Timestamp,
    user: UserId,
    policy_used: RetentionPolicy,
    commits_removed: usize,
    objects_removed: usize,
    space_reclaimed: usize,
    duration: Duration,
}

// View history
eldarin> prune-history
◈ Pruning History
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  2025-01-20 14:32 by root
    Policy: default
    Removed: 2,597 commits, 33,382 objects
    Reclaimed: 4.2 GB
    Duration: 2.3s

  2025-01-15 09:15 by root
    Policy: default
    Removed: 1,234 commits, 15,678 objects
    Reclaimed: 1.8 GB
    Duration: 1.1s
```

#### 3. Pre-Prune Validation

Ensure pruning won't break anything:

```rust
impl Pruner {
    /// Validate pruning operation before execution
    pub fn validate_prune(&self, op: &PruneOperation) -> Result<ValidationReport> {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        // Check 1: Never prune HEAD
        if op.commits_marked.contains(&self.world_tree.head.unwrap()) {
            errors.push("Cannot prune HEAD commit");
        }

        // Check 2: Warn about anchored versions
        let anchored_count = self.count_anchored_in_commits(&op.commits_marked);
        if anchored_count > 0 {
            warnings.push(format!("{} anchored versions will be preserved", anchored_count));
        }

        // Check 3: Ensure no broken references
        for commit_id in &op.commits_marked {
            if self.is_referenced_by_kept_commit(commit_id) {
                errors.push(format!("Commit {} is still referenced", commit_id));
            }
        }

        if !errors.is_empty() {
            Err("Validation failed")
        } else {
            Ok(ValidationReport { warnings, errors })
        }
    }
}
```

---

### Performance Considerations

**Pruning can be expensive:**
- Walking entire commit graph: O(N commits)
- Mark-and-sweep GC: O(N objects)
- Delta compression: O(N × M file size)

**Optimizations:**

```rust
// 1. Incremental pruning (don't do everything at once)
impl Pruner {
    pub fn prune_incremental(&mut self, max_duration: Duration) {
        let start = now();

        while now() - start < max_duration {
            if let Some(commit) = self.get_next_prunable_commit() {
                self.prune_commit(commit);
            } else {
                break;
            }
        }
    }
}

// 2. Background pruning (low-priority thread)
pub fn spawn_pruner_daemon() {
    Loom.spawn(Priority::Idle, || {
        loop {
            sleep(Duration::hours(24));  // Daily pruning

            if disk_usage() > 80% {
                let pruner = get_pruner();
                pruner.prune_by_policy();
            }
        }
    });
}

// 3. Lazy delta compression (compress on-demand)
impl WorldTree {
    pub fn read(&self, file: FileId, cap: &Capability) -> Result<Arc<Vec<u8>>> {
        let metadata = self.metadata.get(&file)?;

        match self.objects.get(&metadata.content_hash)? {
            ObjectStorage::Full(content) => Ok(content.clone()),

            ObjectStorage::Delta { base_hash, diff } => {
                // Reconstruct from delta
                let base = self.read_by_hash(base_hash)?;
                let reconstructed = apply_diff(&base, diff);
                Ok(Arc::new(reconstructed))
            }
        }
    }
}
```

---

### Integration with Implementation Phases

**Phase 1-3:** Core filesystem without pruning
**Phase 4:** Add versioning infrastructure
**Phase 5:** Add capabilities
**Phase 6:** Integration
**Phase 7 (NEW): Pruning & Space Management**

#### Phase 7: Pruning (Week 4)
**Goal:** Prevent unbounded storage growth

**Tasks:**
- [ ] Implement `RetentionPolicy` configuration
- [ ] Implement mark-and-sweep GC
- [ ] Implement time-based commit thinning
- [ ] Add user anchor system
- [ ] Create `prune-stats`, `prune`, `anchor` commands
- [ ] Add prune audit logging
- [ ] Implement validation checks
- [ ] Write tests for edge cases (anchored versions, HEAD protection)

**Optional (Phase 8):**
- [ ] Delta compression for old versions
- [ ] Background pruner daemon
- [ ] Per-file retention policies
- [ ] Compression algorithm optimization

**Deliverable:** Can prune old versions safely while preserving critical history

---

### Example: Space Usage Over Time

**Without Pruning:**
```
Month 1: 100 MB
Month 2: 500 MB
Month 3: 1.2 GB
Month 6: 4.8 GB
Year 1:  12.3 GB  ⚠️ Unsustainable!
```

**With Default Retention Policy:**
```
Month 1: 100 MB
Month 2: 250 MB (weekly snapshots started)
Month 3: 400 MB (monthly snapshots started)
Month 6: 800 MB (old daily versions pruned)
Year 1:  1.2 GB ✓ Stable growth
Year 2:  1.5 GB ✓ Manageable
```

**With Delta Compression:**
```
Year 1:  800 MB  ✓ 60% reduction
Year 2:  1.0 GB  ✓ Space efficient
```

---

## Implementation Phases

### Phase 1: Core Data Structures (Week 1)
**Goal:** Basic RAM storage without queries

- [ ] Define `FileId`, `CommitId`, `SHA256Hash` types
- [ ] Implement `Metadata` struct
- [ ] Implement `Commit` and `Change` structs
- [ ] Create `RamDisk` with HashMap storage
- [ ] Implement `create()` and `read()` operations
- [ ] Write unit tests for basic storage

**Deliverable:** Can create files and read them back by FileId

### Phase 2: Query Language (Week 1-2)
**Goal:** String-based queries work

- [ ] Design query syntax grammar
- [ ] Implement tokenizer
- [ ] Implement parser (string -> AST)
- [ ] Implement query executor
- [ ] Build `QueryIndex` structure
- [ ] Implement index updates on file creation
- [ ] Write tests for various query types

**Deliverable:** Can query files by essence, creator, name, etc.

### Phase 3: Essence Detection (Week 2)
**Goal:** Auto-detect file types

- [ ] Define `Essence` enum hierarchy
- [ ] Implement magic number detection
- [ ] Implement UTF-8 text analysis
- [ ] Implement code language detection
- [ ] Create extensible detector registry
- [ ] Write tests for various file types
- [ ] Document how to add custom essences

**Deliverable:** System correctly identifies file types without extensions

### Phase 4: Versioning (Week 2-3)
**Goal:** Git-like commit history and rollback

- [ ] Implement global commit chain
- [ ] Implement `commit()` operation
- [ ] Track file history through commits
- [ ] Implement `find_file_at_commit()`
- [ ] Implement `rollback_file()`
- [ ] Add temporal queries (created:>date)
- [ ] Write tests for rollback scenarios

**Deliverable:** Can rollback individual files to previous versions

### Phase 5: Capabilities (Week 3)
**Goal:** Security via capabilities

- [ ] Define `Capability` and `Rights` structures
- [ ] Implement capability verification
- [ ] Add capability checks to read/write/delete
- [ ] Implement capability delegation
- [ ] Create capability store
- [ ] Handle capability expiration
- [ ] Write tests for permission scenarios

**Deliverable:** All operations require proper capabilities

### Phase 6: Integration (Week 3-4)
**Goal:** Integrate with existing kernel

- [ ] Create World-Tree Grove interface
- [ ] Integrate with Eldarin shell (shell commands)
- [ ] Add file operations to shell
- [ ] Test from running OS
- [ ] Performance profiling
- [ ] Documentation and examples

**Deliverable:** Can create/query/read files from shell

---

## Open Questions

### 1. Discovery Mechanism
**Status:** Recommended approach chosen (public metadata, private content)
**Next step:** Implement and evaluate in practice

### 2. Persistence
**Question:** How do we persist RAM disk to real disk later?
**Options:**
- Serialize entire structure to single file
- Replicate git's object database structure on disk
- Use existing database format (SQLite)

### 3. Collections/Directories
**Question:** How do we organize files into groups?
**Options:**
- Special "Sanctuary" essence that contains list of FileIds
- Tags only (no collections)
- Virtual collections via saved queries

### 4. Shell Integration
**Question:** What commands should Eldarin provide?
**Proposed:**
```bash
create <name> <content>       # Create file
query <query-string>          # Search files
read <file-id>                # Read file content
write <file-id> <content>     # Modify file
rollback <file-id> <commit>   # Rollback to version
history <file-id>             # Show version history
grant <file-id> <user> <rights>  # Grant capability
```

### 5. Content Search
**Question:** Should we support full-text search?
**Complexity:** Requires indexing file contents, not just metadata
**Defer to:** Phase 7+

---

## Success Criteria

### Phase 1-2 Success
- [ ] Can create files with arbitrary content
- [ ] Can query files by name, essence, creator
- [ ] Unit tests pass

### Phase 3-4 Success
- [ ] System correctly identifies at least 10 different file types
- [ ] Can rollback files to any previous version
- [ ] All versions share storage for unchanged content

### Phase 5-6 Success
- [ ] All operations require valid capabilities
- [ ] Can delegate read-only access
- [ ] Integration tests pass from shell

### Overall Success
- [ ] README example queries work in real shell
- [ ] Performance: <10ms for typical queries
- [ ] Can handle 10,000+ files in RAM disk
- [ ] Zero capability bypass vulnerabilities

---

## References

**Similar Systems:**
- Git (content-addressable storage, commits)
- Fossil SCM (SQLite-based VCS)
- IPFS (content-addressed distributed filesystem)
- Plan 9 (everything-is-a-file, but simpler)
- BeFS (extended attributes, queries)

**Inspiration:**
- [Git Internals](https://git-scm.com/book/en/v2/Git-Internals-Plumbing-and-Porcelain)
- [IPFS Whitepaper](https://ipfs.io/ipfs/QmR7GSQM93Cx5eAg6a6yRzNde1FQv7uL6X1o4k7zrJa3LX)
- [Capability-Based Security](https://en.wikipedia.org/wiki/Capability-based_security)

---

**Next Steps:**
1. Review this plan with stakeholders
2. Begin Phase 1 implementation
3. Update plan based on discoveries during implementation
