/// 2D grid map represented as a bit array.
///
/// We use `false` to represent non-traversable cells and `true` to represent traversable cells.
///
/// The grid map is padded with an additional row above and below, as well as an additional column
/// to the left and right. Attempts to write to these cells will panic, but reads will succeed and
/// return non-traversable.
pub struct BitGrid {
    width: i32,
    height: i32,
    padded_width: usize,
    bits: Box<[u8]>,
}

impl BitGrid {
    #[track_caller]
    pub fn new(width: i32, height: i32) -> Self {
        assert!(width >= 0, "width must be non-negative");
        assert!(height >= 0, "height must be non-negative");
        assert!(width < 2_000_000_000, "width must be < 2000000000");
        assert!(height < 2_000_000_000, "height must be < 2000000000");
        // height + 2 for a padding row above and a padding row below
        // width + 1 for padding column to the left, which also functions as a padding column
        // to the right, except for the last row which requires an extra bit
        let padded_width = (width + 1) as usize;
        let bits = padded_width
            .checked_mul((height + 2) as usize)
            // +1 for the bottom-right corner bit
            .and_then(|b| b.checked_add(8))
            .expect("number of bits in grid exceeds usize::MAX");
        // extra padding for u64 reads
        let bytes = 8 + bits / 8 + 8;
        BitGrid {
            width,
            height,
            padded_width,
            bits: vec![0; bytes].into_boxed_slice(),
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

    #[track_caller]
    #[inline(always)]
    pub fn get(&self, x: i32, y: i32) -> bool {
        self.padded_bounds_check(x, y);
        unsafe { self.get_unchecked(x, y) }
    }

    #[track_caller]
    #[inline(always)]
    pub fn set(&mut self, x: i32, y: i32, traversable: bool) {
        self.unpadded_bounds_check(x, y);
        unsafe {
            self.set_unchecked(x, y, traversable);
        }
    }

    /// Gets the traversability of a cell without bounds checking.
    ///
    /// # Safety
    /// The coordinates must be in-bounds of the padded grid. Specifically:
    /// - `x` is in `-1..=self.width()`
    /// - `y` is in `-1..=self.height()`
    #[inline(always)]
    #[cfg_attr(debug_assertions, track_caller)]
    pub unsafe fn get_unchecked(&self, x: i32, y: i32) -> bool {
        #[cfg(debug_assertions)]
        self.padded_bounds_check(x, y);
        let (byte, bit) = self.index(x, y);
        unsafe {
            // SAFETY: The caller is responsible for ensuring that the coordinates are in-bounds.
            *self.bits.get_unchecked(byte) & 1 << bit != 0
        }
    }

    /// Sets the traversability of a cell without bounds checking.
    ///
    /// # Safety
    /// The coordinates must be in-bounds of the grid. Specifically:
    /// - `x` is in `0..self.width()`
    /// - `y` is in `0..self.height()`
    #[inline(always)]
    #[cfg_attr(debug_assertions, track_caller)]
    pub unsafe fn set_unchecked(&mut self, x: i32, y: i32, traversable: bool) {
        #[cfg(debug_assertions)]
        self.unpadded_bounds_check(x, y);
        let (byte, bit) = self.index(x, y);
        unsafe {
            // SAFETY: The caller is responsible for ensuring that the coordinates are in-bounds.
            *self.bits.get_unchecked_mut(byte) &= !(1 << bit);
            *self.bits.get_unchecked_mut(byte) |= (traversable as u8) << bit;
        }
    }

    /// Gets the traversability of a row of cells to the right without bounds checking.
    ///
    /// The traversability of the requested cell is placed in the least significant bit, with the
    /// traversability of the cell right of it in the second-least significant bit, the cell to the
    /// right of that in the third least-significant bit, etc.
    ///
    /// Depending on the position of the bit for the requested cell within its byte, between 57 and
    /// 64 cells of information may be returned, with the remaining bits being 0. Additionally, the
    /// values of bits corresponding to cells outside of the padded grid are unspecified.
    ///
    /// # Safety
    /// The coordinates must be in-bounds of the padded grid. Specifically:
    /// - `x` is in `-1..=self.width()`
    /// - `y` is in `-1..=self.height()`
    #[inline(always)]
    #[cfg_attr(debug_assertions, track_caller)]
    pub unsafe fn get_row_right(&self, x: i32, y: i32) -> u64 {
        #[cfg(debug_assertions)]
        self.padded_bounds_check(x, y);
        let (byte, bit) = self.index(x, y);
        let raw_bits = u64::from_le_bytes(unsafe {
            self.bits.get_unchecked(byte..byte + 8).try_into().unwrap()
        });
        raw_bits >> bit
    }

    /// Gets the traversability of a row of cells to the left without bounds checking.
    ///
    /// The traversability of the requested cell is placed in the most significant bit, with the
    /// traversability of the cell right of it in the second-most significant bit, the cell to the
    /// right of that in the third-most significant bit, etc.
    ///
    /// Depending on the position of the bit for the requested cell within its byte, between 57 and
    /// 64 cells of information may be returned, with the remaining bits being 0. Additionally, the
    /// values of bits corresponding to cells outside of the padded grid are unspecified.
    ///
    /// # Safety
    /// The coordinates must be in-bounds of the padded grid. Specifically:
    /// - `x` is in `-1..=self.width()`
    /// - `y` is in `-1..=self.height()`
    #[inline(always)]
    #[cfg_attr(debug_assertions, track_caller)]
    pub unsafe fn get_row_left(&self, x: i32, y: i32) -> u64 {
        #[cfg(debug_assertions)]
        self.padded_bounds_check(x, y);
        let (byte, bit) = self.index(x, y);
        let raw_bits = u64::from_le_bytes(unsafe {
            self.bits.get_unchecked(byte-7..byte + 1).try_into().unwrap()
        });
        raw_bits << (7 - bit)
    }

    #[track_caller]
    #[inline(always)]
    fn padded_bounds_check(&self, x: i32, y: i32) {
        assert!(x >= -1, "x out of bounds");
        assert!(y >= -1, "y out of bounds");
        assert!(x <= self.width, "x out of bounds");
        assert!(y <= self.height, "y out of bounds");
    }

    #[track_caller]
    #[inline(always)]
    fn unpadded_bounds_check(&self, x: i32, y: i32) {
        assert!(x >= 0, "x out of bounds");
        assert!(y >= 0, "y out of bounds");
        assert!(x < self.width, "x out of bounds");
        assert!(y < self.height, "y out of bounds");
    }

    #[inline(always)]
    fn index(&self, x: i32, y: i32) -> (usize, usize) {
        let padded_y = (y + 1) as usize;
        let padded_x = (x + 1) as usize;
        let bit = padded_x + padded_y * self.padded_width;
        (bit / 8 + 8, bit % 8)
    }
}
