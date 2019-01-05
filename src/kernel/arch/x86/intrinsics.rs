//! Special Instruction Intrinsics
//!
//! The x86 architecture includes a wide variety of specialized instructions
//! which may be rather inconvenient to use directly. Here wrappers are provide
//! for instructions that are useful to many areas of the kernel.  Instructions
//! specific to a single subsystem are better left safely wrapped in the
//! relevant modules.

/// Transmits byte to port
#[inline(always)]
pub fn outb(port: u16, data: u8) {
    unsafe { asm!("out dx, al" :: "{dx}"(port),"{al}"(data) :: "volatile","intel") }
}

/// Receives byte from port
#[inline(always)]
pub fn inb(port: u16) -> u8 {
    let data;
    unsafe { asm!("in al, dx" : "={al}"(data) : "{dx}"(port) :: "volatile","intel") }
    data
}

/// Reads model-specific register
#[inline(always)]
pub fn rdmsr(register: u32) -> u64 {
    let (hi, lo): (u64, u64);
    unsafe { asm!("rdmsr" : "={eax}"(lo),"={edx}"(hi) : "{ecx}"(register) :: "intel" ) }
    (hi << 32) | lo
}

/// Writes model-specific register
#[inline(always)]
pub fn wrmsr(register: u32, value: u64) {
    let (hi, lo) = (value >> 32, value & 0xffff_ffff);
    unsafe { asm!("wrmsr" :: "{ecx}"(register),"{eax}"(lo),"{edx}"(hi) :: "intel" ) }
}

/// Sets bit in model-specific register
#[inline(always)]
pub fn stmsr(register: u32, offset: usize) {
    let value = rdmsr(register);
    wrmsr(register, value | (1 << offset));
}

/// Halts execution permanently for this core
///
/// This disables interrupts then blocks indefinitely on the next interrupt.
/// This may be interrupted by NMIs, hence the loop.
#[inline(always)]
pub fn halt() -> ! {
    unsafe { asm!("0: cli; hlt; jmp 0b") }
    loop { } // compiler hint about divergence
}

/// Permanent record of cpuid results
static mut CPUID_RESULTS: Option<CpuidResults> = None;

/// Returns a pointer the cpuid results
pub fn get_cpuid() -> &'static CpuidResults {
    unsafe { // smp safe because this should occur early on BSP
        if CPUID_RESULTS.is_none() {
            CPUID_RESULTS = Some(CpuidResults::new());
        }
        CPUID_RESULTS.as_ref().unwrap()
    }
}

/// Execute the cpuid instruction
///
/// Prefer `get_cpuid()` to access already cached results
pub fn cpuid(eax: u32, ecx: u32) -> CpuidRegs {
    let mut regs = CpuidRegs::new();
    unsafe {
        asm!("cpuid "
            :"={eax}"(regs.eax),"={ebx}"(regs.ebx)
            ,"={ecx}"(regs.ecx),"={edx}"(regs.edx)
            :"{eax}"(eax), "{ecx}"(ecx)
            :: "volatile"
        );
    }
    regs
}

#[derive(Copy, Clone, Debug)]
pub struct CpuidRegs {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

impl CpuidRegs {
    /// Returns a zero initialized structure
    pub fn new() -> Self {
        CpuidRegs { eax: 0, ebx: 0, ecx: 0, edx: 0 }
    }
}

pub struct CpuidResults {
    pub supported: bool,
    pub base:  [Option<CpuidRegs>; 0x18],
    pub extra: [Option<CpuidRegs>; 0x08],
    vendor_id: Option<[u8; 12]>, // save demangled result
    vendor: Option<CpuVendor>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CpuVendor {
    Intel,
    AMD,
}

macro_rules! flag (
    ($name:ident =  $region:ident[$i:expr].$reg:ident.$b:expr) => (pub fn $name(&self) -> bool {
        self.$region[$i].as_ref().map(|r| ((r.$reg >> $b) & 1) == 1).unwrap_or(false)
    })
);

macro_rules! field (
    ($name:ident =  $region:ident[$i:expr].$reg:ident.$e:expr,$s:expr) => (pub fn $name(&self) -> Option<u32> {
        self.$region[$i].as_ref().map(|r| (((r.$reg & ((1 << $e) - 1))) >> $s))
    })
);

const CPUID_EXTRA: u32 = 0x80000000;
impl CpuidResults {
    unsafe fn query_base(&mut self, eax: u32) {
        self.base[eax as usize] = Some(cpuid(eax, 0));
    }
    unsafe fn query_extra(&mut self, eax: u32) {
        self.extra[eax as usize] = Some(cpuid(CPUID_EXTRA + eax, 0));
    }
    unsafe fn new() -> Self {
        let mut c = CpuidResults {
            supported: true,
            base:  [None; 0x18],
            extra: [None; 0x08],
            vendor_id: None,
            vendor: None,
        };

        let supported: u64;
        // attempt detecting if CPUID instruction is supported by enabling in %eflags
        // Intel volume 1 is a little vague on these semantics
        asm!("
        pushfq
        pop %rax
        bts $$21, %rax
        push %rax
        popf
        pushfq
        pop %rax
        ":"={rax}"(supported)::"~rax","~rflags","~memory":"volatile");
        if (supported & (1 << 21)) == 0 {
            c.supported = false;
            return c
        }

        c.query_base(0);
        let leaves = c.base[0].unwrap().eax;
        for i in 1 .. leaves.min(c.base.len() as u32) {
            c.query_base(i);
        }

        c.query_extra(0);
        let leaves = c.extra[0].unwrap().eax;
        for i in 1 .. leaves.wrapping_sub(CPUID_EXTRA).min(c.extra.len() as u32) {
            c.query_extra(i);
        }

        c.init_vendor_id();
        c.init_vendor();

        c
    }
    unsafe fn init_vendor_id(&mut self) {
        if let Some(ref leaf) = self.base[0] {
            self.vendor_id = Some([b' '; 12]);
            let out = self.vendor_id.as_mut().unwrap() as *mut [u8; 12] as *mut u32;
            *out.offset(0) = leaf.ebx;
            *out.offset(1) = leaf.edx;
            *out.offset(2) = leaf.ecx;
        }
    }
    fn init_vendor(&mut self) {
        self.vendor = match self.vendor_id() {
            Some("GenuineIntel") => Some(CpuVendor::Intel),
            Some("AuthenticAMD") => Some(CpuVendor::AMD),
            _ => None
        };
    }
    pub fn vendor_id(&self) -> Option<&str> {
        use core::str::from_utf8;
        self.vendor_id.as_ref().map(|p| from_utf8(p)).and_then(Result::ok)
    }
    pub fn vendor(&self) -> Option<CpuVendor> {
        self.vendor
    }

    flag!(x2apic  = base[1].ecx.21);
    flag!(pse     = base[1].edx.3);
    flag!(msr     = base[1].edx.5);
    flag!(pae     = base[1].edx.6);
    flag!(apic    = base[1].edx.9);

    flag!(rdpid   = base[7].ecx.22);

    flag!(syscall = extra[1].edx.11);
    flag!(page1gb = extra[1].edx.26);
    flag!(rdtscp  = extra[1].edx.27);

    field!(stepping = base[1].eax.3,0);
    field!(model    = base[1].eax.7,4);
    field!(family   = base[1].eax.11,8);
    field!(extended_model  = base[1].eax.19,16);
    field!(extended_family = base[1].eax.27,20);
    pub fn effective_model(&self) -> Option<u32> {
        let (f, m) = (self.family()?, self.model()?);
        match f {
            6 | 15 => Some(m + (self.extended_model()? << 4)),
            _      => Some(m),
        }
    }
    pub fn effective_family(&self) -> Option<u32> {
        let f = self.family()?;
        match f {
            15 => Some(f + self.extended_family()?),
            _  => Some(f),
        }
    }
}
