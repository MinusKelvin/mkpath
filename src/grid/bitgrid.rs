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
        // to the right, except for the last row which requires an
        let padded_width = (width + 1) as usize;
        let bits = padded_width
            .checked_mul((height + 2) as usize)
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
        let bit = padded_x + padded_y * self.padded_width as usize;
        (bit / 8 + 8, bit % 8)
    }
}
