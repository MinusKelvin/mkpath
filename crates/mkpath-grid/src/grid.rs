pub struct Grid<T> {
    width: i32,
    height: i32,
    cells: Box<[T]>,
}

impl<T> Grid<T> {
    pub fn new(width: i32, height: i32, mut f: impl FnMut(i32, i32) -> T) -> Self {
        let w: usize = width.try_into().expect("width must be non-negative");
        let h: usize = height.try_into().expect("height must be non-negative");
        let cells = (0..h)
            .flat_map(move |x| (0..w).map(move |y| (x as i32, y as i32)))
            .map(|(x, y)| f(x, y))
            .collect();
        Grid {
            width,
            height,
            cells,
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn storage(&self) -> &[T] {
        &self.cells
    }

    pub fn storage_mut(&mut self) -> &mut [T] {
        &mut self.cells
    }

    /// Returns a reference to a cell of the grid, without bounds checking.
    ///
    /// # Safety
    /// The coordinates must be in-bounds of the grid. Specifically:
    /// - `x` is in `0..self.width()`
    /// - `y` is in `0..self.height()`
    #[inline(always)]
    #[cfg_attr(debug_assertions, track_caller)]
    pub unsafe fn get_unchecked(&self, x: i32, y: i32) -> &T {
        #[cfg(debug_assertions)]
        self.bounds_check(x, y);
        unsafe { self.cells.get_unchecked(self.index(x, y)) }
    }

    /// Returns a mutable reference to a cell of the grid, without bounds checking.
    ///
    /// # Safety
    /// The coordinates must be in-bounds of the grid. Specifically:
    /// - `x` is in `0..self.width()`
    /// - `y` is in `0..self.height()`
    #[inline(always)]
    #[cfg_attr(debug_assertions, track_caller)]
    pub unsafe fn get_unchecked_mut(&mut self, x: i32, y: i32) -> &mut T {
        #[cfg(debug_assertions)]
        self.bounds_check(x, y);
        unsafe { self.cells.get_unchecked_mut(self.index(x, y)) }
    }

    #[inline(always)]
    fn index(&self, x: i32, y: i32) -> usize {
        self.width as usize * y as usize + x as usize
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

impl<T> std::ops::Index<(i32, i32)> for Grid<T> {
    type Output = T;

    #[track_caller]
    fn index(&self, (x, y): (i32, i32)) -> &T {
        self.bounds_check(x, y);
        unsafe { self.get_unchecked(x, y) }
    }
}

impl<T> std::ops::IndexMut<(i32, i32)> for Grid<T> {
    fn index_mut(&mut self, (x, y): (i32, i32)) -> &mut T {
        self.bounds_check(x, y);
        unsafe { self.get_unchecked_mut(x, y) }
    }
}
