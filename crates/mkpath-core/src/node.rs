use std::alloc::Layout;
use std::cell::Cell;
use std::marker::PhantomData;
use std::process::abort;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};

use bumpalo::Bump;

pub struct NodeBuilder {
    layout_id: LayoutId,
    layout: Layout,
    default: Vec<u8>,
}

#[derive(Clone, Copy)]
pub struct NodeRef<'a> {
    ptr: NonNull<Node>,
    _marker: PhantomData<Cell<&'a ()>>,
}

/// Pointee type for pointers to nodes.
pub enum Node {}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LayoutId(u64);

#[derive(Clone, Copy)]
struct NodeHeader {
    layout_id: LayoutId,
    parent: Option<NonNull<Node>>,
}

#[derive(Clone, Copy)]
pub struct NodeMemberPointer<T: Copy> {
    layout_id: LayoutId,
    offset: usize,
    _marker: PhantomData<T>,
}

pub struct NodeAllocator {
    layout_id: LayoutId,
    default: Box<[u8]>,
    layout: Layout,
    arena: Bump,
}

static LAYOUT_ID: AtomicU64 = AtomicU64::new(0);

impl LayoutId {
    fn new() -> Self {
        let layout_id = LAYOUT_ID.fetch_add(1, Ordering::SeqCst);
        if layout_id > i64::MAX as u64 {
            // Safety can be violated if layout ids end up shared, so if we exceed i64::MAX
            // layout_ids then we need to abort the process to prevent this. This is the same
            // strategy Arc uses to avoid a similar issue, and is not technically fool-proof;
            // theoretically, after reaching i64::MAX layouts, i64::MAX new layouts could be
            // created between the atomic increment and the abort causing the u64 to wrap, but this
            // seems incredibly unlikely. We don't expect this abort to ever happen in practice,
            // since constructing i64::MAX layouts would both take a very long time and be a
            // symptom of misuse of the library. You're supposed to keep your node allocator around.
            abort();
        }
        LayoutId(layout_id)
    }
}

impl NodeBuilder {
    pub fn new() -> NodeBuilder {
        let layout_id = LayoutId::new();
        let layout = Layout::new::<NodeHeader>();
        let mut default = vec![];
        default.resize(layout.size(), 0);
        unsafe {
            // We use `write_unaligned` here because the vector holding the default value has no
            // alignment guarantees.
            // SAFETY: The buffer is sized appropriately to store a `NodeHeader` object.
            default
                .as_mut_ptr()
                .cast::<NodeHeader>()
                .write_unaligned(NodeHeader {
                    layout_id,
                    parent: None,
                });
        }
        NodeBuilder {
            layout_id,
            default,
            layout,
        }
    }

    pub fn build(self) -> NodeAllocator {
        let layout = self.layout.pad_to_align();
        let mut default = self.default;
        default.resize(layout.size(), 0);
        NodeAllocator {
            layout_id: self.layout_id,
            default: default.into_boxed_slice(),
            layout,
            arena: Bump::new(),
        }
    }

    pub fn add_field<T: Copy + 'static>(&mut self, default: T) -> NodeMemberPointer<T> {
        let (layout, offset) = self.layout.extend(Layout::new::<T>()).unwrap();
        self.default.resize(layout.size(), 0);
        unsafe {
            // SAFETY: The buffer is sized according to `layout` and the offset refers to a field
            //         of the `layout`, and so this must be in-bounds of the buffer.
            self.default
                .as_mut_ptr()
                .add(offset)
                .cast::<T>()
                // We use `write_unaligned` here because the vector holding the default value has no
                // alignment guarantees.
                // SAFETY: The buffer is sized according to `layout` and the pointer points to where
                //         the layout specifies an object of type `T` exists, so this is fine. We
                //         do not drop the old `T`, but `T: Copy` so this is also fine.
                .write_unaligned(default);
        }
        self.layout = layout;
        NodeMemberPointer {
            layout_id: self.layout_id,
            offset,
            _marker: PhantomData,
        }
    }
}

impl NodeAllocator {
    pub fn reset(&mut self) {
        self.arena.reset();
    }

    pub fn generate_node<'a>(&'a self) -> NodeRef<'a> {
        let ptr = self.arena.alloc_layout(self.layout);
        unsafe {
            // SAFETY: We have the invariant that `self.default` is valid bytes for initializing a
            //         node, which means it is sized appropriately.
            std::ptr::copy_nonoverlapping(self.default.as_ptr(), ptr.as_ptr(), self.layout.size());
        }
        NodeRef {
            ptr: ptr.cast(),
            _marker: PhantomData,
        }
    }

    pub fn layout_id(&self) -> LayoutId {
        self.layout_id
    }
}

impl<'a> NodeRef<'a> {
    #[track_caller]
    #[inline(always)]
    pub fn get<T: Copy + 'static>(self, f: NodeMemberPointer<T>) -> T {
        self.check_layout(f.layout_id);
        // SAFETY: We have checked that the member pointer is for the layout `self` has.
        unsafe { self.get_unchecked(f) }
    }

    #[track_caller]
    #[inline(always)]
    pub fn set<T: Copy + 'static>(self, f: NodeMemberPointer<T>, value: T) {
        self.check_layout(f.layout_id);
        // SAFETY: We have checked that the member pointer is for the layout `self` has.
        unsafe { self.set_unchecked(f, value) }
    }

    #[inline(always)]
    pub fn layout_id(self) -> LayoutId {
        // SAFETY: All NodeRefs start with a `NodeHeader` struct, so the resulting reference refers
        // to a valid `NodeHeader`.
        unsafe { &*self.ptr.as_ptr().cast::<NodeHeader>() }.layout_id
    }

    #[inline(always)]
    pub fn get_parent(self) -> Option<NodeRef<'a>> {
        unsafe { &*self.ptr.as_ptr().cast::<NodeHeader>() }
            .parent
            .map(|ptr| NodeRef {
                ptr,
                _marker: PhantomData,
            })
    }

    #[inline(always)]
    pub fn set_parent(self, parent: Option<NodeRef<'a>>) {
        unsafe { &mut *self.ptr.as_ptr().cast::<NodeHeader>() }.parent = parent.map(|ptr| ptr.ptr);
    }

    /// Gets the specified field without compatibility checking.
    ///
    /// # Safety
    /// The caller must ensure that the `NodeMemberPointer` has the same layout id as `self`.
    #[cfg_attr(debug_assertions, track_caller)]
    #[inline(always)]
    pub unsafe fn get_unchecked<T: Copy + 'static>(self, f: NodeMemberPointer<T>) -> T {
        #[cfg(debug_assertions)]
        self.check_layout(f.layout_id);
        // SAFETY: Since `f` is for the layout of this node, there exists an object of type T at
        //         the specified offset from this node's pointer.
        unsafe { self.ptr.as_ptr().cast::<u8>().add(f.offset).cast::<T>().read() }
    }

    /// Sets the specified field without compatibility checking.
    ///
    /// # Safety
    /// The caller must ensure that the `NodeMemberPointer` has the same layout id as `self`.
    #[cfg_attr(debug_assertions, track_caller)]
    #[inline(always)]
    pub unsafe fn set_unchecked<T: Copy + 'static>(self, f: NodeMemberPointer<T>, value: T) {
        #[cfg(debug_assertions)]
        self.check_layout(f.layout_id);
        // We do not need to drop the existing object because `T: Copy`.
        // SAFETY: Since `f` is for the layout of this node, there exists an object of type T at
        //         the specified offset from this node's pointer.
        unsafe { self.ptr.as_ptr().cast::<u8>().add(f.offset).cast::<T>().write(value) }
    }

    #[inline(always)]
    pub fn same_ptr(self, other: NodeRef) -> bool {
        self.ptr == other.ptr
    }

    #[inline(always)]
    pub fn raw(self) -> NonNull<Node> {
        self.ptr
    }

    /// Constructs a `NodeRef` from a raw pointer.
    ///
    /// # Safety
    /// The pointer must have been previously returned by a call to `NodeRef::raw`, and the
    /// underlying `NodeRef` must not have been freed. The caller should also be careful that the
    /// lifetime of the returned `NodeRef` does not exceed the actual lifetime of the data.
    #[inline(always)]
    pub unsafe fn from_raw(ptr: NonNull<Node>) -> Self {
        NodeRef {
            ptr,
            _marker: PhantomData,
        }
    }

    #[track_caller]
    #[inline(always)]
    fn check_layout(&self, layout_id: LayoutId) {
        if self.layout_id() != layout_id {
            panic!("mismatched layout");
        }
    }
}

impl<T: Copy> NodeMemberPointer<T> {
    #[inline(always)]
    pub fn layout_id(&self) -> LayoutId {
        self.layout_id
    }
}
