//! # ELF Object File Writer
//!
//! Generates ELF64 object files from compiled code.
//!
//! ELF (Executable and Linkable Format) is the standard binary format for
//! Unix-like systems, including AethelOS. This module creates object files
//! that can be linked into executables or loaded as Ring 1/3 services.
//!
//! ## ELF64 Structure
//!
//! ```text
//! ┌─────────────────┐
//! │  ELF Header     │  64 bytes
//! ├─────────────────┤
//! │  Program        │  (for executables)
//! │  Headers        │
//! ├─────────────────┤
//! │  .text section  │  Code
//! ├─────────────────┤
//! │  .data section  │  Initialized data
//! ├─────────────────┤
//! │  .bss section   │  Uninitialized data
//! ├─────────────────┤
//! │  .symtab        │  Symbol table
//! ├─────────────────┤
//! │  .strtab        │  String table
//! ├─────────────────┤
//! │  .shstrtab      │  Section name strings
//! ├─────────────────┤
//! │  Section        │
//! │  Headers        │
//! └─────────────────┘
//! ```

use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// ELF file class
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum ElfClass {
    None = 0,
    Elf32 = 1,
    Elf64 = 2,
}

/// ELF data encoding
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum ElfData {
    None = 0,
    LittleEndian = 1,
    BigEndian = 2,
}

/// ELF file type
#[repr(u16)]
#[derive(Debug, Clone, Copy)]
pub enum ElfType {
    None = 0,
    Relocatable = 1,     // .o files
    Executable = 2,      // executables
    Shared = 3,          // .so files
    Core = 4,            // core dumps
}

/// ELF machine type
#[repr(u16)]
#[derive(Debug, Clone, Copy)]
pub enum ElfMachine {
    None = 0,
    X86 = 3,
    X86_64 = 62,
}

/// Section type
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum SectionType {
    Null = 0,
    ProgBits = 1,        // Program data
    SymTab = 2,          // Symbol table
    StrTab = 3,          // String table
    Rela = 4,            // Relocation entries with addends
    Hash = 5,            // Symbol hash table
    Dynamic = 6,         // Dynamic linking info
    Note = 7,            // Notes
    NoBits = 8,          // .bss (no file space)
    Rel = 9,             // Relocation entries without addends
}

/// Section flags
#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum SectionFlags {
    Write = 0x1,         // Writable
    Alloc = 0x2,         // Occupies memory
    ExecInstr = 0x4,     // Executable
}

/// Symbol binding
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum SymbolBinding {
    Local = 0,
    Global = 1,
    Weak = 2,
}

/// Symbol type
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum SymbolType {
    NoType = 0,
    Object = 1,
    Func = 2,
    Section = 3,
    File = 4,
}

/// ELF64 Header (64 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Header {
    pub e_ident: [u8; 16],       // Magic number and other info
    pub e_type: u16,             // Object file type
    pub e_machine: u16,          // Architecture
    pub e_version: u32,          // Object file version
    pub e_entry: u64,            // Entry point virtual address
    pub e_phoff: u64,            // Program header table file offset
    pub e_shoff: u64,            // Section header table file offset
    pub e_flags: u32,            // Processor-specific flags
    pub e_ehsize: u16,           // ELF header size in bytes
    pub e_phentsize: u16,        // Program header table entry size
    pub e_phnum: u16,            // Program header table entry count
    pub e_shentsize: u16,        // Section header table entry size
    pub e_shnum: u16,            // Section header table entry count
    pub e_shstrndx: u16,         // Section header string table index
}

/// ELF64 Section Header (64 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64SectionHeader {
    pub sh_name: u32,            // Section name (string tbl index)
    pub sh_type: u32,            // Section type
    pub sh_flags: u64,           // Section flags
    pub sh_addr: u64,            // Section virtual addr at execution
    pub sh_offset: u64,          // Section file offset
    pub sh_size: u64,            // Section size in bytes
    pub sh_link: u32,            // Link to another section
    pub sh_info: u32,            // Additional section information
    pub sh_addralign: u64,       // Section alignment
    pub sh_entsize: u64,         // Entry size if section holds table
}

/// ELF64 Symbol Table Entry (24 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Symbol {
    pub st_name: u32,            // Symbol name (string tbl index)
    pub st_info: u8,             // Symbol type and binding
    pub st_other: u8,            // Symbol visibility
    pub st_shndx: u16,           // Section index
    pub st_value: u64,           // Symbol value
    pub st_size: u64,            // Symbol size
}

impl Elf64Header {
    /// Create a new ELF64 header for a relocatable object file
    pub fn new_relocatable() -> Self {
        let mut e_ident = [0u8; 16];
        e_ident[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);  // Magic number
        e_ident[4] = ElfClass::Elf64 as u8;                         // 64-bit
        e_ident[5] = ElfData::LittleEndian as u8;                   // Little endian
        e_ident[6] = 1;                                             // ELF version
        e_ident[7] = 0;                                             // System V ABI

        Elf64Header {
            e_ident,
            e_type: ElfType::Relocatable as u16,
            e_machine: ElfMachine::X86_64 as u16,
            e_version: 1,
            e_entry: 0,
            e_phoff: 0,
            e_shoff: 0,  // Will be set later
            e_flags: 0,
            e_ehsize: core::mem::size_of::<Elf64Header>() as u16,
            e_phentsize: 0,
            e_phnum: 0,
            e_shentsize: core::mem::size_of::<Elf64SectionHeader>() as u16,
            e_shnum: 0,  // Will be set later
            e_shstrndx: 0,  // Will be set later
        }
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.e_ident);
        bytes.extend_from_slice(&self.e_type.to_le_bytes());
        bytes.extend_from_slice(&self.e_machine.to_le_bytes());
        bytes.extend_from_slice(&self.e_version.to_le_bytes());
        bytes.extend_from_slice(&self.e_entry.to_le_bytes());
        bytes.extend_from_slice(&self.e_phoff.to_le_bytes());
        bytes.extend_from_slice(&self.e_shoff.to_le_bytes());
        bytes.extend_from_slice(&self.e_flags.to_le_bytes());
        bytes.extend_from_slice(&self.e_ehsize.to_le_bytes());
        bytes.extend_from_slice(&self.e_phentsize.to_le_bytes());
        bytes.extend_from_slice(&self.e_phnum.to_le_bytes());
        bytes.extend_from_slice(&self.e_shentsize.to_le_bytes());
        bytes.extend_from_slice(&self.e_shnum.to_le_bytes());
        bytes.extend_from_slice(&self.e_shstrndx.to_le_bytes());
        bytes
    }
}

impl Elf64SectionHeader {
    /// Create a null section header
    pub fn null() -> Self {
        Elf64SectionHeader {
            sh_name: 0,
            sh_type: SectionType::Null as u32,
            sh_flags: 0,
            sh_addr: 0,
            sh_offset: 0,
            sh_size: 0,
            sh_link: 0,
            sh_info: 0,
            sh_addralign: 0,
            sh_entsize: 0,
        }
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.sh_name.to_le_bytes());
        bytes.extend_from_slice(&self.sh_type.to_le_bytes());
        bytes.extend_from_slice(&self.sh_flags.to_le_bytes());
        bytes.extend_from_slice(&self.sh_addr.to_le_bytes());
        bytes.extend_from_slice(&self.sh_offset.to_le_bytes());
        bytes.extend_from_slice(&self.sh_size.to_le_bytes());
        bytes.extend_from_slice(&self.sh_link.to_le_bytes());
        bytes.extend_from_slice(&self.sh_info.to_le_bytes());
        bytes.extend_from_slice(&self.sh_addralign.to_le_bytes());
        bytes.extend_from_slice(&self.sh_entsize.to_le_bytes());
        bytes
    }
}

impl Elf64Symbol {
    /// Create a null symbol
    pub fn null() -> Self {
        Elf64Symbol {
            st_name: 0,
            st_info: 0,
            st_other: 0,
            st_shndx: 0,
            st_value: 0,
            st_size: 0,
        }
    }

    /// Create a function symbol
    pub fn function(name_idx: u32, section_idx: u16, value: u64, size: u64) -> Self {
        Elf64Symbol {
            st_name: name_idx,
            st_info: (SymbolBinding::Global as u8) << 4 | (SymbolType::Func as u8),
            st_other: 0,
            st_shndx: section_idx,
            st_value: value,
            st_size: size,
        }
    }

    /// Convert to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.st_name.to_le_bytes());
        bytes.push(self.st_info);
        bytes.push(self.st_other);
        bytes.extend_from_slice(&self.st_shndx.to_le_bytes());
        bytes.extend_from_slice(&self.st_value.to_le_bytes());
        bytes.extend_from_slice(&self.st_size.to_le_bytes());
        bytes
    }
}

/// String table builder
pub struct StringTable {
    strings: Vec<u8>,
}

impl StringTable {
    /// Create a new string table
    pub fn new() -> Self {
        let mut st = StringTable {
            strings: Vec::new(),
        };
        st.strings.push(0);  // First entry is always empty string
        st
    }

    /// Add a string and return its index
    pub fn add(&mut self, s: &str) -> u32 {
        let index = self.strings.len() as u32;
        self.strings.extend_from_slice(s.as_bytes());
        self.strings.push(0);  // Null terminator
        index
    }

    /// Get the string table bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        self.strings.clone()
    }

    /// Get the size
    pub fn len(&self) -> usize {
        self.strings.len()
    }
}

/// ELF object file builder
pub struct ElfBuilder {
    text_section: Vec<u8>,
    data_section: Vec<u8>,
    symbols: Vec<Elf64Symbol>,
    string_table: StringTable,
    shstring_table: StringTable,
}

impl ElfBuilder {
    /// Create a new ELF builder
    pub fn new() -> Self {
        let mut symbols = Vec::new();
        symbols.push(Elf64Symbol::null());  // First symbol is always null

        ElfBuilder {
            text_section: Vec::new(),
            data_section: Vec::new(),
            symbols,
            string_table: StringTable::new(),
            shstring_table: StringTable::new(),
        }
    }

    /// Add code to .text section
    pub fn add_text(&mut self, code: &[u8]) {
        self.text_section.extend_from_slice(code);
    }

    /// Add data to .data section
    pub fn add_data(&mut self, data: &[u8]) {
        self.data_section.extend_from_slice(data);
    }

    /// Add a function symbol
    pub fn add_function(&mut self, name: &str, offset: u64, size: u64) {
        let name_idx = self.string_table.add(name);
        let symbol = Elf64Symbol::function(name_idx, 1, offset, size);  // Section 1 is .text
        self.symbols.push(symbol);
    }

    /// Build the final ELF file
    pub fn build(&mut self) -> Vec<u8> {
        let mut output = Vec::new();

        // Create header
        let mut header = Elf64Header::new_relocatable();

        // Build section name string table
        let null_name = self.shstring_table.add("");
        let text_name = self.shstring_table.add(".text");
        let data_name = self.shstring_table.add(".data");
        let bss_name = self.shstring_table.add(".bss");
        let symtab_name = self.shstring_table.add(".symtab");
        let strtab_name = self.shstring_table.add(".strtab");
        let shstrtab_name = self.shstring_table.add(".shstrtab");

        // Calculate offsets
        let text_offset = core::mem::size_of::<Elf64Header>() as u64;
        let data_offset = text_offset + self.text_section.len() as u64;
        let symtab_offset = data_offset + self.data_section.len() as u64;

        let symtab_bytes: Vec<u8> = self.symbols.iter()
            .flat_map(|sym| sym.to_bytes())
            .collect();

        let strtab_offset = symtab_offset + symtab_bytes.len() as u64;
        let strtab_bytes = self.string_table.to_bytes();

        let shstrtab_offset = strtab_offset + strtab_bytes.len() as u64;
        let shstrtab_bytes = self.shstring_table.to_bytes();

        let section_headers_offset = shstrtab_offset + shstrtab_bytes.len() as u64;

        // Update header
        header.e_shoff = section_headers_offset;
        header.e_shnum = 7;  // null, .text, .data, .bss, .symtab, .strtab, .shstrtab
        header.e_shstrndx = 6;  // .shstrtab is section 6

        // Write header
        output.extend_from_slice(&header.to_bytes());

        // Write sections
        output.extend_from_slice(&self.text_section);
        output.extend_from_slice(&self.data_section);
        output.extend_from_slice(&symtab_bytes);
        output.extend_from_slice(&strtab_bytes);
        output.extend_from_slice(&shstrtab_bytes);

        // Write section headers
        // 0: Null section
        output.extend_from_slice(&Elf64SectionHeader::null().to_bytes());

        // 1: .text
        let text_header = Elf64SectionHeader {
            sh_name: text_name,
            sh_type: SectionType::ProgBits as u32,
            sh_flags: (SectionFlags::Alloc as u64) | (SectionFlags::ExecInstr as u64),
            sh_addr: 0,
            sh_offset: text_offset,
            sh_size: self.text_section.len() as u64,
            sh_link: 0,
            sh_info: 0,
            sh_addralign: 16,
            sh_entsize: 0,
        };
        output.extend_from_slice(&text_header.to_bytes());

        // 2: .data
        let data_header = Elf64SectionHeader {
            sh_name: data_name,
            sh_type: SectionType::ProgBits as u32,
            sh_flags: (SectionFlags::Alloc as u64) | (SectionFlags::Write as u64),
            sh_addr: 0,
            sh_offset: data_offset,
            sh_size: self.data_section.len() as u64,
            sh_link: 0,
            sh_info: 0,
            sh_addralign: 8,
            sh_entsize: 0,
        };
        output.extend_from_slice(&data_header.to_bytes());

        // 3: .bss (empty for now)
        let bss_header = Elf64SectionHeader {
            sh_name: bss_name,
            sh_type: SectionType::NoBits as u32,
            sh_flags: (SectionFlags::Alloc as u64) | (SectionFlags::Write as u64),
            sh_addr: 0,
            sh_offset: 0,
            sh_size: 0,
            sh_link: 0,
            sh_info: 0,
            sh_addralign: 8,
            sh_entsize: 0,
        };
        output.extend_from_slice(&bss_header.to_bytes());

        // 4: .symtab
        let symtab_header = Elf64SectionHeader {
            sh_name: symtab_name,
            sh_type: SectionType::SymTab as u32,
            sh_flags: 0,
            sh_addr: 0,
            sh_offset: symtab_offset,
            sh_size: symtab_bytes.len() as u64,
            sh_link: 5,  // Link to .strtab
            sh_info: 1,  // One local symbol (null)
            sh_addralign: 8,
            sh_entsize: core::mem::size_of::<Elf64Symbol>() as u64,
        };
        output.extend_from_slice(&symtab_header.to_bytes());

        // 5: .strtab
        let strtab_header = Elf64SectionHeader {
            sh_name: strtab_name,
            sh_type: SectionType::StrTab as u32,
            sh_flags: 0,
            sh_addr: 0,
            sh_offset: strtab_offset,
            sh_size: strtab_bytes.len() as u64,
            sh_link: 0,
            sh_info: 0,
            sh_addralign: 1,
            sh_entsize: 0,
        };
        output.extend_from_slice(&strtab_header.to_bytes());

        // 6: .shstrtab
        let shstrtab_header = Elf64SectionHeader {
            sh_name: shstrtab_name,
            sh_type: SectionType::StrTab as u32,
            sh_flags: 0,
            sh_addr: 0,
            sh_offset: shstrtab_offset,
            sh_size: shstrtab_bytes.len() as u64,
            sh_link: 0,
            sh_info: 0,
            sh_addralign: 1,
            sh_entsize: 0,
        };
        output.extend_from_slice(&shstrtab_header.to_bytes());

        output
    }
}

/// Create an ELF object file from machine code
pub fn create_elf_object(code: &[u8], function_name: &str) -> Vec<u8> {
    let mut builder = ElfBuilder::new();
    builder.add_text(code);
    builder.add_function(function_name, 0, code.len() as u64);
    builder.build()
}
