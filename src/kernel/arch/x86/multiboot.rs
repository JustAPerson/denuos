/// Multiboot Parsing
///
/// Multiboot describes a protocol for transferring control from a bootloader to
/// an operating system. Denuos uses GRUB2 to load. GRUB is responsible for
/// reading our entire kernel image from disk, loading it into memory, and
/// retrieving critical information from the BIOS before transferring control to
/// us.
///
/// All of this critical information is stored in a tagged data structure. When
/// GRUB calls our entry point (see start32.s), a pointer to this struct is in
/// the EBX register. Consider this a pointer to the MultibootTags struct.
use core;
use core::fmt;

/// Pointer to the Multiboot tag structure
#[repr(C)]
pub struct MultibootTags {
    size: u32,
    reserved: u32,
}

/// Wrapper around useful Multiboot tags
#[derive(Debug, Default)]
pub struct MultibootInfo {
    pub cmd_line:         Option<&'static str>,
    pub boot_loader_name: Option<&'static str>,
    pub basic_mem_info:   Option<&'static BasicMemInfo>,
    pub bios_boot_dev:    Option<&'static BiosBootDevice>,
    pub mem_map:          Option<&'static [MMapEntry]>,
    pub elf_sections:     Option<ElfSections>,
}

/// Helper to parse individual multiboot tags
struct Tag {
    ty: u32,
    size: u32,
}

impl MultibootTags {
    /// Parse the Multiboot tags into a MultibootInfo
    ///
    /// Unsupported tags will be silently ignored. Only fields present in the
    /// MultibootInfo struct are currently supported.
    pub unsafe fn parse(&self) -> MultibootInfo {
        let mut info = MultibootInfo::default();
        let mut tag: *const Tag = self.start() as *const Tag;
        let limit = (self.end() + 1) as *const Tag; // point just past the last valid tag

        tag = tag.offset(1);
        while tag < limit {
            let tag_size = (*tag).size as usize;
            let data = tag.offset(1) as usize;
            let data_size = tag_size - 8;

            match (*tag).ty {
                0 => { } // End tag
                1 => {
                    // Boot command line
                    let s = parse_tag_str(data, data_size).expect("Non-utf8 boot command line");
                    info.cmd_line = Some(s);
                }
                2 => {
                    // Boot loader name
                    let s = parse_tag_str(data, data_size).expect("Non-utf8 boot loader name");
                    info.boot_loader_name = Some(s);
                }
                3 => { } // NYI Modules
                4 => {
                    // Basic memory info
                    let basic = &*(data as *const BasicMemInfo);
                    info.basic_mem_info = Some(basic);
                }
                5 => {
                    // BIOS Boot Device
                    let bootdev = &*(data as *const BiosBootDevice);
                    info.bios_boot_dev = Some(bootdev);
                }
                6 => {
                    // Memory Map
                    let entry_size    = *(data as *const u32);
                    let entry_version = *((data + 4) as *const u32);
                    assert!(entry_size == 24 && entry_version == 0, "Unsupported bootloader");

                    let entries = (data + 8) as *const MMapEntry;
                    let n = data_size / entry_size as usize;

                    info.mem_map = Some(core::slice::from_raw_parts(entries, n));
                }
                7 => { } // VBE
                8 => { } // framebuffer
                9 => {
                    // elf sections
                    let num =     *(data as *const u32) as usize;
                    let entsize = *((data + 4) as *const u32) as usize;
                    let shndx =   *((data + 8) as *const u32) as usize;

                    let ptr = (data + 12) as *const ElfSection;
                    // exclude string name tables
                    let list = core::slice::from_raw_parts(ptr, shndx);

                    info.elf_sections = Some(ElfSections {
                        num:     num,
                        list:    list,
                        entsize: entsize,
                        shndx:   shndx,
                    });
                }
                10 => { } // APM
                11 => { } // EFI32
                12 => { } // EFI64
                13 => { } // SMBIOS
                14 => { } // ACPI Old
                15 => { } // ACPI New
                16 => { } // Network
                17 => { } // EFI MMap
                18 => { } // EFI BS
                i => panic!("Corrupt MultibootInfo Tag: {}", i)
            }

            let new_tag = (tag as usize) + tag_size;
            tag = ((new_tag + 7) & !7) as *const Tag; // round to 8 byte alignment
            // end tag already 8 byte aligned, so assertion below won't fail
        }
        assert!(tag == limit, "Corrupt MultibootInfo");

        info
    }

    /// Return pointer to beginning of the structure
    pub fn start(&self) -> usize {
        self as *const _ as usize
    }

    /// Returns pointer to the last byte of the structure
    pub fn end(&self) -> usize {
        self.start() + self.size as usize - 1
    }
}

/// Parses a null-terminated string from a tag
unsafe fn parse_tag_str(data: usize, data_size: usize) -> Option<&'static str> {
    let ptr = data as *const u8;
    let size = data_size - 1; // subtract null terminator
    let bytes = core::slice::from_raw_parts(ptr, size);
    core::str::from_utf8(bytes).ok()
}


#[repr(C)]
pub struct BiosBootDevice {
    pub biosdev: u32,
    partition: u32,
    sub_partition: u32,
}

#[repr(C)]
pub struct BasicMemInfo {
    pub mem_lower: u32,
    pub mem_upper: u32,
}

#[repr(C)]
pub struct MMapEntry {
    pub base_addr: u64,
    pub length:    u64,
    pub ty:        MMapEntryType,
    reserved:      u32,
}

#[repr(u32)]
#[derive(Debug, Eq, PartialEq)]
pub enum MMapEntryType {
    Free     = 1,
    Reserved = 2,
    ACPI     = 3,
    Preserve = 4,
    Bad      = 5,
}

/// List of ELF sections
#[repr(C)]
#[derive(Debug)]
pub struct ElfSections {
    pub num:  usize,
    pub list: &'static [ElfSection],
    entsize:  usize,
    shndx:    usize,
}

/// Limited wrapper around ELF64 sections
#[repr(C)]
#[derive(Debug)]
pub struct ElfSection {
    sh_name:      u32,
    sh_type:      u32,
    sh_flags:     u64,
    sh_addr:      u64,
    sh_offset:    u64,
    sh_size:      u64,
    sh_link:      u32,
    sh_info:      u32,
    sh_addralign: u64,
    sh_entsize:   u64,
}

impl ElfSections {
    /// Return pointer to start of kernel image
    pub fn image_start(&self) -> usize {
        self.list.iter().filter(|s| s.is_allocated()).map(|s| s.start()).min().unwrap()
    }

    /// Return size of kernel image
    pub fn image_size(&self) -> usize {
        self.list.iter().filter(|s| s.is_allocated()).map(|s| s.size()).sum()
    }

    /// Return pointer to the last byte of kernel image
    pub fn image_end(&self) -> usize {
        self.list.iter().filter(|s| s.is_allocated()).map(|s| s.end()).max().unwrap()
    }
}

impl ElfSection {
    /// Has this section been loaded into memory?
    pub fn is_allocated(&self) -> bool {
        self.sh_flags & 0x2 != 0
    }

    /// Return pointer to section
    pub fn start(&self) -> usize {
        self.sh_addr as usize
    }

    /// Return size of section
    pub fn size(&self) -> usize {
        self.sh_size as usize
    }

    /// Return pointer to the last byte of section
    pub fn end(&self) -> usize {
        self.start() + self.size() - 1
    }
}

impl BiosBootDevice {
    pub fn partition(&self) -> Option<u32> {
        if self.partition == !0 {
            return None;
        }
        Some(self.partition)
    }

    pub fn sub_partition(&self) -> Option<u32> {
        if self.sub_partition == !0 {
            return None;
        }
        Some(self.sub_partition)
    }
}

impl MMapEntry {
    pub fn is_free(&self) -> bool {
        self.ty == MMapEntryType::Free
    }
}

impl fmt::Debug for BiosBootDevice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BiosBootDevice {{ biosdev: 0x{:x}, partition: 0x{:x}, sub_partition: 0x{:x} }}",
               self.biosdev, self.partition, self.sub_partition)
    }
}

impl fmt::Debug for BasicMemInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BasicMemInfo {{ mem_lower: 0x{:x}, mem_upper: 0x{:x} }}",
               self.mem_lower, self.mem_upper)
    }
}

impl fmt::Debug for MMapEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MMapEntry {{ base_addr: 0x{:x}, length: 0x{:x}, ty: {:?} }}",
               self.base_addr, self.length, self.ty)
    }
}

