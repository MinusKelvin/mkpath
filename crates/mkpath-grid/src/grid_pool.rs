use std::cell::Cell;
use std::ptr::NonNull;

use mkpath_core::traits::NodePool;
use mkpath_core::{Node, NodeAllocator, NodeMemberPointer, NodeRef};

use super::grid::Grid;
use super::GridNodePool;

pub struct GridPool {
    state_map: Grid<Cell<(u64, *mut Node)>>,
    search_number: u64,
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
        assert!(
            allocator.layout_id() == state_field.layout_id(),
            "mismatched layouts"
        );

        GridPool {
            search_number: 1,
            state_map: Grid::new(width, height, |_, _| Cell::new((0, std::ptr::null_mut()))),
            state_field,
            allocator,
        }
    }

    #[inline(always)]
    pub fn width(&self) -> i32 {
        self.state_map.width()
    }

    #[inline(always)]
    pub fn height(&self) -> i32 {
        self.state_map.height()
    }

    #[track_caller]
    #[inline(always)]
    pub fn get(&self, state: (i32, i32)) -> Option<NodeRef> {
        let _ = self.state_map[state];
        unsafe { self.get_unchecked(state) }
    }

    /// Retrieves the node for the specified state or generates one if it does not exist, without
    /// performing bounds checks.
    ///
    /// # Safety
    /// The coordinates must be in-bounds of the grid. Specifically:
    /// - `x` is in `0..self.width()`
    /// - `y` is in `0..self.height()`
    #[inline(always)]
    #[cfg_attr(debug_assertions, track_caller)]
    pub unsafe fn generate_unchecked(&self, (x, y): (i32, i32)) -> NodeRef {
        let slot = unsafe { self.state_map.get_unchecked(x, y) };
        let (num, ptr) = slot.get();
        if num == self.search_number {
            debug_assert!(!ptr.is_null());
            unsafe { NodeRef::from_raw(NonNull::new_unchecked(ptr)) }
        } else {
            let ptr = self.allocator.new_node();
            unsafe {
                ptr.set_unchecked(self.state_field, (x, y));
            }
            slot.set((self.search_number, ptr.into_raw().as_ptr()));
            ptr
        }
    }

    /// Retrieves the node for the specified state if it exists, without performing bounds checks.
    ///
    /// # Safety
    /// The coordinates must be in-bounds of the grid. Specifically:
    /// - `x` is in `0..self.width()`
    /// - `y` is in `0..self.height()`
    #[inline(always)]
    #[cfg_attr(debug_assertions, track_caller)]
    pub unsafe fn get_unchecked(&self, (x, y): (i32, i32)) -> Option<NodeRef> {
        let slot = unsafe { self.state_map.get_unchecked(x, y) };
        let (num, ptr) = slot.get();
        if num == self.search_number {
            unsafe { Some(NodeRef::from_raw(NonNull::new_unchecked(ptr))) }
        } else {
            None
        }
    }
}

impl NodePool for GridPool {
    type State = (i32, i32);

    fn reset(&mut self) {
        self.search_number = self.search_number.checked_add(1).unwrap_or_else(|| {
            self.state_map
                .storage_mut()
                .fill(Cell::new((0, std::ptr::null_mut())));
            1
        });
        self.allocator.reset();
    }

    fn generate(&self, state: Self::State) -> NodeRef {
        let _ = self.state_map[state];
        unsafe { self.generate_unchecked(state) }
    }
}

impl GridNodePool for GridPool {
    fn width(&self) -> i32 {
        self.width()
    }

    fn height(&self) -> i32 {
        self.height()
    }

    unsafe fn generate_unchecked(&self, state: (i32, i32)) -> NodeRef {
        // SAFETY: Forwarding call to function with identical contract.
        unsafe { self.generate_unchecked(state) }
    }
}
