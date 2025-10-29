/// FATE - Manage the Concordance of Fates (RBAC system)
///
/// This command provides runtime management of Roles (Fates) and Subject
/// assignments, similar to grsecurity's gradm utility.
///
/// Commands:
///   fate list           - List all defined Fates
///   fate show <name>    - Show details of a specific Fate
///   fate subjects       - List all Subjects and their assigned Fates
///   fate assign <id> <fate> - Assign a Fate to a Subject
///   fate seal           - Seal the Concordance (make immutable)
///   fate status         - Show Concordance status

use crate::mana_pool::concordance_of_fates::{self, SubjectId, SubjectType};
extern crate alloc;
use alloc::vec::Vec;

pub fn cmd_fate(args_str: &str) {
    // Split args string into Vec<&str>
    let args: Vec<&str> = if args_str.trim().is_empty() {
        Vec::new()
    } else {
        args_str.trim().split_whitespace().collect()
    };

    if args.is_empty() {
        show_usage();
        return;
    }

    match args[0] {
        "list" => cmd_fate_list(),
        "show" => {
            if args.len() < 2 {
                crate::println!("Usage: fate show <fate-name>");
                return;
            }
            cmd_fate_show(args[1]);
        }
        "subjects" => cmd_fate_subjects(),
        "assign" => {
            if args.len() < 3 {
                crate::println!("Usage: fate assign <subject-id> <fate-name>");
                return;
            }
            cmd_fate_assign(args[1], args[2]);
        }
        "seal" => cmd_fate_seal(),
        "status" => cmd_fate_status(),
        _ => {
            crate::println!("Unknown fate command: {}", args[0]);
            show_usage();
        }
    }
}

fn show_usage() {
    crate::println!("◈ Fate - Manage the Concordance of Fates");
    crate::println!();
    crate::println!("Commands:");
    crate::println!("  fate list              List all defined Fates");
    crate::println!("  fate show <name>       Show details of a specific Fate");
    crate::println!("  fate subjects          List all Subjects and their Fates");
    crate::println!("  fate assign <id> <fate> Assign a Fate to a Subject");
    crate::println!("  fate seal              Seal the Concordance (make immutable)");
    crate::println!("  fate status            Show Concordance status");
    crate::println!();
    crate::println!("The Concordance defines what each entity can and cannot do.");
    crate::println!("Once sealed, Fates cannot be modified - only assigned.");
}

fn cmd_fate_status() {
    crate::println!("◈ Concordance of Fates Status");
    crate::println!();

    if !concordance_of_fates::is_concordance_active() {
        crate::println!("  ⚠ Concordance not initialized");
        return;
    }

    let sealed = concordance_of_fates::is_sealed();
    let fate_count = concordance_of_fates::get_fate_count();
    let subject_count = concordance_of_fates::get_subject_count();

    crate::println!("  Status: {}", if sealed { "✓ Sealed (immutable)" } else { "○ Unsealed (mutable)" });
    crate::println!("  Fates defined: {}", fate_count);
    crate::println!("  Subjects bound: {}", subject_count);
    crate::println!();

    if sealed {
        crate::println!("  The Concordance is sealed. No new Fates may be defined.");
        crate::println!("  The destinies are written. They cannot be changed.");
    } else {
        crate::println!("  The Concordance remains unsealed. Fates may still be defined.");
        crate::println!("  Use 'fate seal' to make the Concordance immutable.");
    }
}

fn cmd_fate_list() {
    crate::println!("◈ Defined Fates in the Concordance");
    crate::println!();

    if !concordance_of_fates::is_concordance_active() {
        crate::println!("  ⚠ Concordance not initialized");
        return;
    }

    unsafe {
        let concordance = concordance_of_fates::get_concordance();

        if concordance.fate_count() == 0 {
            crate::println!("  No Fates defined.");
            return;
        }

        for (name, fate) in concordance.fates.iter() {
            let privilege_mark = if fate.is_privileged { "★" } else { "○" };
            crate::println!("  {} {}", privilege_mark, name);
            crate::println!("     {}", fate.description);

            // Show key capabilities
            let caps = &fate.capabilities;
            let mut cap_str = alloc::string::String::new();
            if caps.can_read_files { cap_str.push_str("read "); }
            if caps.can_write_files { cap_str.push_str("write "); }
            if caps.can_execute_files { cap_str.push_str("exec "); }
            if caps.can_fork { cap_str.push_str("fork "); }
            if caps.can_bind_network { cap_str.push_str("network "); }

            if !cap_str.is_empty() {
                crate::println!("     Capabilities: {}", cap_str.trim());
            }

            crate::println!();
        }
    }
}

fn cmd_fate_show(fate_name: &str) {
    crate::println!("◈ Fate Details: {}", fate_name);
    crate::println!();

    if !concordance_of_fates::is_concordance_active() {
        crate::println!("  ⚠ Concordance not initialized");
        return;
    }

    unsafe {
        let concordance = concordance_of_fates::get_concordance();

        if let Some(fate) = concordance.fates.get(fate_name) {
            crate::println!("  Name: {}", fate.name);
            crate::println!("  Description: {}", fate.description);
            crate::println!("  Privileged: {}", if fate.is_privileged { "Yes (kernel level)" } else { "No (user level)" });
            crate::println!();

            crate::println!("  Capabilities:");
            let caps = &fate.capabilities;
            crate::println!("    File Operations:");
            crate::println!("      Read:    {}", if caps.can_read_files { "✓" } else { "✗" });
            crate::println!("      Write:   {}", if caps.can_write_files { "✓" } else { "✗" });
            crate::println!("      Execute: {}", if caps.can_execute_files { "✓" } else { "✗" });
            crate::println!("      Create:  {}", if caps.can_create_files { "✓" } else { "✗" });
            crate::println!("      Delete:  {}", if caps.can_delete_files { "✓" } else { "✗" });
            crate::println!();

            crate::println!("    Process Operations:");
            crate::println!("      Fork:   {}", if caps.can_fork { "✓" } else { "✗" });
            crate::println!("      Exec:   {}", if caps.can_exec { "✓" } else { "✗" });
            crate::println!("      Kill:   {}", if caps.can_kill { "✓" } else { "✗" });
            crate::println!();

            crate::println!("    Network Operations:");
            crate::println!("      Bind:    {}", if caps.can_bind_network { "✓" } else { "✗" });
            crate::println!("      Connect: {}", if caps.can_connect_network { "✓" } else { "✗" });
            crate::println!("      Listen:  {}", if caps.can_listen_network { "✓" } else { "✗" });
            crate::println!();

            crate::println!("    Security:");
            crate::println!("      Read Symbols: {}", if caps.can_read_symbols { "✓" } else { "✗" });
            crate::println!("      Load Modules: {}", if caps.can_load_modules { "✓" } else { "✗" });
            crate::println!("      Modify System: {}", if caps.can_modify_system { "✓" } else { "✗" });
            crate::println!();

            if !fate.file_rules.is_empty() {
                crate::println!("  File Rules:");
                for rule in &fate.file_rules {
                    let perm = match rule.permission {
                        crate::mana_pool::concordance_of_fates::Permission::Allow => "✓ Allow",
                        crate::mana_pool::concordance_of_fates::Permission::Deny => "✗ Deny",
                    };
                    crate::println!("    {} {:?} on {}", perm, rule.access_type, rule.path_pattern);
                }
                crate::println!();
            }

            if !fate.allowed_transitions.is_empty() {
                crate::println!("  Allowed Fate Transitions:");
                for transition in &fate.allowed_transitions {
                    crate::println!("    → {}", transition);
                }
                crate::println!();
            }
        } else {
            crate::println!("  Fate '{}' not found in the Concordance.", fate_name);
        }
    }
}

fn cmd_fate_subjects() {
    crate::println!("◈ Subjects in the Concordance");
    crate::println!();

    if !concordance_of_fates::is_concordance_active() {
        crate::println!("  ⚠ Concordance not initialized");
        return;
    }

    unsafe {
        let concordance = concordance_of_fates::get_concordance();

        if concordance.subject_count() == 0 {
            crate::println!("  No Subjects registered.");
            return;
        }

        crate::println!("  ID    Type           Fate");
        crate::println!("  ────  ─────────────  ──────────────");

        for (id, subject) in concordance.subjects.iter() {
            let type_str = match subject.subject_type {
                SubjectType::KernelThread => "Kernel Thread",
                SubjectType::UserProcess => "User Process ",
                SubjectType::SystemService => "System Service",
            };

            crate::println!("  {:4}  {}  {}", id.0, type_str, subject.fate);
        }
        crate::println!();
    }
}

fn cmd_fate_assign(subject_id_str: &str, fate_name: &str) {
    if !concordance_of_fates::is_concordance_active() {
        crate::println!("⚠ Concordance not initialized");
        return;
    }

    // Parse subject ID
    let subject_id = match subject_id_str.parse::<u64>() {
        Ok(id) => SubjectId(id),
        Err(_) => {
            crate::println!("Error: Invalid subject ID '{}'. Must be a number.", subject_id_str);
            return;
        }
    };

    unsafe {
        let concordance = concordance_of_fates::get_concordance();

        match concordance.assign_fate(subject_id, fate_name) {
            Ok(_) => {
                crate::println!("✓ Subject {} assigned to Fate '{}'", subject_id.0, fate_name);
                crate::println!("  The destiny is written.");
            }
            Err(e) => {
                crate::println!("✗ Failed to assign Fate: {}", e);
            }
        }
    }
}

fn cmd_fate_seal() {
    if !concordance_of_fates::is_concordance_active() {
        crate::println!("⚠ Concordance not initialized");
        return;
    }

    unsafe {
        let concordance = concordance_of_fates::get_concordance();

        if concordance.is_sealed() {
            crate::println!("The Concordance is already sealed.");
            return;
        }

        crate::println!("◈ Sealing the Concordance of Fates");
        crate::println!();
        crate::println!("  Once sealed, no new Fates may be defined.");
        crate::println!("  The destinies will be immutable.");
        crate::println!();
        crate::println!("  Are you certain? This cannot be undone!");
        crate::println!("  [This is automatic - confirming seal...]");
        crate::println!();

        concordance.seal();

        crate::println!("  ✓ The Concordance is sealed.");
        crate::println!("  ✓ The fates are written in stone.");
        crate::println!("  ✓ No entity may escape its destiny.");
    }
}
