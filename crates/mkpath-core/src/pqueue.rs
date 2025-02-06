use std::cmp::Reverse;

use crate::node::*;
use crate::traits::OpenList;

/// Factory for creating [`PriorityQueue`]s for a node layout.
pub struct PriorityQueueFactory {
    index: NodeMemberPointer<usize>,
}

pub struct PriorityQueue<'a, C> {
    cmp: C,
    index: NodeMemberPointer<usize>,
    // We have the invariant that all NodeRefs in this heap have the same layout as index and cmp.
    heap: Vec<NodeRef<'a>>,
}

/// Trait for ordering `NodeRef`s by their field(s).
///
/// # Safety
/// If `Self::compatible_layout` returns true for a layout id, then it must be safe to pass
/// `NodeRef`s with that layout id to `Self::le_unchecked`.
pub unsafe trait FieldComparator {
    /// Perform `<=` comparison.
    ///
    /// # Safety
    /// The caller must ensure that the layout ids of the `NodeRef`s cause
    /// `Self::compatible_layout` to return true.
    unsafe fn le_unchecked(&self, lhs: NodeRef, rhs: NodeRef) -> bool;

    fn compatible_layout(&self, layout_id: LayoutId) -> bool;
}

impl PriorityQueueFactory {
    pub fn new(builder: &mut NodeBuilder) -> Self {
        PriorityQueueFactory {
            index: builder.add_field(usize::MAX),
        }
    }

    pub fn new_queue<'a, C: FieldComparator>(&mut self, cmp: C) -> PriorityQueue<'a, C> {
        assert!(cmp.compatible_layout(self.index.layout_id()));
        PriorityQueue {
            cmp,
            index: self.index,
            heap: vec![],
        }
    }
}

impl<'a, C: FieldComparator> OpenList<'a> for PriorityQueue<'a, C> {
    fn relaxed(&mut self, node: NodeRef<'a>) {
        let index = node.get(self.index);
        if index >= self.heap.len() || !self.heap[index].ptr_eq(node) {
            self.heap.push(node);
            unsafe {
                self.sift_up(node, self.heap.len() - 1);
            }
        } else {
            unsafe {
                self.sift_up(node, index);
            }
        }
    }

    fn next(&mut self) -> Option<NodeRef<'a>> {
        if self.heap.is_empty() {
            return None;
        }
        let ret = self.heap.swap_remove(0);
        if let Some(&node) = self.heap.first() {
            unsafe {
                self.sift_down(node, 0);
            }
        }
        Some(ret)
    }
}

impl<'a, C: FieldComparator> PriorityQueue<'a, C> {
    unsafe fn sift_up(&mut self, node: NodeRef<'a>, mut index: usize) {
        unsafe {
            while index > 0 {
                let parent_index = (index - 1) / 2;
                let parent = *self.heap.get_unchecked(parent_index);
                if self.cmp.le_unchecked(parent, node) {
                    break;
                }
                *self.heap.get_unchecked_mut(index) = parent;
                parent.set_unchecked(self.index, index);
                index = parent_index;
            }
            *self.heap.get_unchecked_mut(index) = node;
            node.set_unchecked(self.index, index);
        }
    }

    unsafe fn sift_down(&mut self, node: NodeRef<'a>, mut index: usize) {
        unsafe {
            loop {
                let child_1_index = index * 2 + 1;
                if child_1_index >= self.heap.len() {
                    break;
                }
                let child_1 = self.heap[child_1_index];

                let child_index;
                let child;

                let child_2_index = child_1_index + 1;
                if child_2_index < self.heap.len() {
                    let child_2 = self.heap[child_2_index];

                    if self.cmp.le_unchecked(child_1, child_2) {
                        child_index = child_1_index;
                        child = child_1;
                    } else {
                        child_index = child_2_index;
                        child = child_2;
                    }
                } else {
                    child_index = child_1_index;
                    child = child_1;
                }

                if self.cmp.le_unchecked(node, child) {
                    break;
                }

                *self.heap.get_unchecked_mut(index) = child;
                child.set_unchecked(self.index, index);
                index = child_index;
            }

            *self.heap.get_unchecked_mut(index) = node;
            node.set_unchecked(self.index, index);
        }
    }
}

unsafe impl<T: PartialOrd + Copy + 'static> FieldComparator for NodeMemberPointer<T> {
    unsafe fn le_unchecked(&self, lhs: NodeRef, rhs: NodeRef) -> bool {
        unsafe { lhs.get_unchecked(*self) <= rhs.get_unchecked(*self) }
    }

    fn compatible_layout(&self, layout_id: LayoutId) -> bool {
        self.layout_id() == layout_id
    }
}

unsafe impl<T: PartialOrd + Copy + 'static> FieldComparator for Reverse<NodeMemberPointer<T>> {
    unsafe fn le_unchecked(&self, lhs: NodeRef, rhs: NodeRef) -> bool {
        unsafe { lhs.get_unchecked(self.0) >= rhs.get_unchecked(self.0) }
    }

    fn compatible_layout(&self, layout_id: LayoutId) -> bool {
        self.0.compatible_layout(layout_id)
    }
}

macro_rules! tuple_fieldcmp_impl {
    ($($typ:ident $index:tt)*) => {
        unsafe impl<$($typ: FieldComparator),*> FieldComparator for ($($typ,)*) {
            unsafe fn le_unchecked(&self, lhs: NodeRef, rhs: NodeRef) -> bool {
                tuple_fieldcmp_impl!(@cmp self lhs rhs $($index)*)
            }

            fn compatible_layout(&self, layout_id: LayoutId) -> bool {
                $(self.$index.compatible_layout(layout_id))&&*
            }
        }
    };
    (@cmp $self:ident $lhs:ident $rhs:ident $last:tt) => {
        unsafe { $self.$last.le_unchecked($lhs, $rhs) }
    };
    (@cmp $self:ident $lhs:ident $rhs:ident $next:tt $($rest:tt)+) => {{
        let l_leq_r = unsafe { $self.$next.le_unchecked($lhs, $rhs) };
        let r_leq_l = unsafe { $self.$next.le_unchecked($rhs, $lhs) };
        if l_leq_r && r_leq_l {
            tuple_fieldcmp_impl!(@cmp $self $lhs $rhs $($rest)*)
        } else {
            l_leq_r
        }
    }};
}

tuple_fieldcmp_impl!(A 0);
tuple_fieldcmp_impl!(A 0 B 1);
tuple_fieldcmp_impl!(A 0 B 1 C 2);
tuple_fieldcmp_impl!(A 0 B 1 C 2 D 3);
tuple_fieldcmp_impl!(A 0 B 1 C 2 D 3 E 4);
tuple_fieldcmp_impl!(A 0 B 1 C 2 D 3 E 4 F 5);
