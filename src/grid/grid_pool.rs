use std::cell::Cell;
use std::ptr::NonNull;

use crate::node::{NodeAllocator, NodeMemberPointer, NodeRef};

pub struct GridPool {
    width: i32,
    height: i32,
    search_number: u64,
    state_map: Box<[Cell<(u64, *mut u8)>]>,
    state_field: NodeMemberPointer<(i32, i32)>,
    allocator: NodeAllocator,
}

impl GridPool {
    #[track_caller]
    pub fn new(
        allocator: NodeAllocator,
        state_field: NodeMemberPointer<(i32, i32)>,
        width: i32,
        height: i32,
    ) -> Self {
        assert!(width >= 0, "width must be non-negative");
        assert!(height >= 0, "height must be non-negative");
        assert!(allocator.same_layout(state_field), "mismatched layouts");
        let num = (width as usize)
            .checked_mul(height as usize)
            .expect("width*height exceeds usize::MAX");
        GridPool {
            width,
            height,
            search_number: 1,
            state_map: vec![Cell::new((0, std::ptr::null_mut())); num].into_boxed_slice(),
            state_field,
            allocator,
        }
    }

    #[inline(always)]
    pub fn width(&self) -> i32 {
        self.width
    }

    #[inline(always)]
    pub fn height(&self) -> i32 {
        self.height
    }

    #[inline(always)]
    pub fn state_member(&self) -> NodeMemberPointer<(i32, i32)> {
        self.state_field
    }

    pub fn reset(&mut self) {
        self.search_number = self.search_number.checked_add(1).unwrap_or_else(|| {
            self.state_map.fill(Cell::new((0, std::ptr::null_mut())));
            1
        });
        self.allocator.reset();
    }

    #[track_caller]
    #[inline(always)]
    pub fn generate(&self, x: i32, y: i32) -> NodeRef {
        self.bounds_check(x, y);
        unsafe { self.generate_unchecked(x, y) }
    }

    #[inline(always)]
    #[cfg_attr(debug_assertions, track_caller)]
    pub unsafe fn generate_unchecked(&self, x: i32, y: i32) -> NodeRef {
        #[cfg(debug_assertions)]
        self.bounds_check(x, y);
        let slot = self
            .state_map
            .get_unchecked(x as usize + y as usize * self.width as usize);
        let (num, ptr) = slot.get();
        if num == self.search_number {
            debug_assert!(!ptr.is_null());
            unsafe { NodeRef::from_raw(NonNull::new_unchecked(ptr)) }
        } else {
            let ptr = self.allocator.generate_node();
            unsafe {
                ptr.set_unchecked(self.state_field, (x, y));
            }
            slot.set((self.search_number, ptr.raw().as_ptr()));
            ptr
        }
    }

    #[track_caller]
    #[inline(always)]
    fn bounds_check(&self, x: i32, y: i32) {
        assert!(x >= 0, "x out of bounds");
        assert!(y >= 0, "y out of bounds");
        assert!(x < self.width, "x out of bounds");
        assert!(y < self.height, "y out of bounds");
    }
}
