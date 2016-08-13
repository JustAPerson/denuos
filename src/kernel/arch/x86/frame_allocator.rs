//! Simple Page Frame Allocator
//!
//! A `Frame` contains the physical memory that may be mapped by a virtual
//! page. We are given a memory map from the `MultibootInfo`. This defines the
//! regions of memory that are safe for use. Currently we are only concerned
//! with a unique allocation of frames. Reuse is unsupported.  A frame is valid
//! if it is page aligned, in a free memory region, and it is does not overlap
//! a protected region. Protected regions are used to avoid overwriting certain
//! structures until a better memory mapping can be established.

use super::multiboot::MMapEntry;

/// The size in bytes of a normal page
const PAGE_SIZE: usize = 4096;

/// Defines a the first and last byte of a region
pub type MemRegion = (usize, usize);

/// A simplistic frame allocator that provides access to a supply of
/// unique frames.
///
/// A list of "protected regions" may be supplied. No frames provided
/// will overlap with these regions.
pub struct FrameAllocator {
    start: usize,
    end:   usize,
    protected_regions: &'static [MemRegion],
}

/// A unique reference to a physical memory page.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Frame {
    index: usize,
}

impl FrameAllocator {
    pub fn new(mem_regions: &'static [MMapEntry],
               protected_regions: &'static [MemRegion]) -> FrameAllocator {
        let free_region = mem_regions.iter().filter(|r| r.is_free())
                                     .max_by_key(|r| r.size())
                                     .expect("No usable memory");

        let allocator = FrameAllocator {
            start: Frame::after(free_region.start()).addr(),
            end: Frame::containing(free_region.end()).addr(),
            protected_regions: protected_regions,
        };
        allocator
    }

    /// Allocate a unique Frame
    pub fn alloc(&mut self) -> Frame {
        'verify_frame: loop {
            let next_page = self.next_page().expect("Out of memory");
            for region in self.protected_regions {
                let start = Frame::containing(region.0);
                let end   = Frame::containing(region.1);

                if next_page >= start && next_page <= end {
                    continue 'verify_frame;
                }
            }

            return next_page
        }
    }

    /// Deallocate a Frame. Currently NYI.
    pub fn free(&mut self) {
        // TODO NYI
    }

    /// Approximate the remaining number of pages.
    /// Does not consider protected regions.
    pub fn free_pages(&self) -> usize {
        (self.end - self.start) / PAGE_SIZE + 1
    }

    fn next_page(&mut self) -> Option<Frame> {
        if self.start >= self.end { return None; }
        let addr = self.start;
        self.start += PAGE_SIZE;
        Some(Frame::containing(addr))
    }
}


impl Frame {
    /// Get address to the start of this frame
    pub fn addr(&self) -> usize {
        self.index * PAGE_SIZE
    }

    /// Get the Frame containing this address
    /// ```
    /// Frame::containing(0x00FF).addr() // 0x0000
    /// ```
    fn containing(addr: usize) -> Frame {
        Frame { index: addr / PAGE_SIZE }
    }

    /// Round up to the next Frame if necessary
    ///
    /// For example, If a region starts in the middle of a frame, then
    /// we're only really interested in the first valid frame after this.
    /// ```
    /// Frame::after(0x0000).addr() // 0x0000
    /// Frame::after(0x1000).addr() // 0x1000
    /// Frame::after(0x1001).addr() // 0x2000
    /// ```
    fn after(addr: usize) -> Frame {
        const MASK: usize = PAGE_SIZE - 1;
        let addr_rounded_up = (addr + MASK) & !MASK;
        Frame::containing(addr_rounded_up)
    }
}
