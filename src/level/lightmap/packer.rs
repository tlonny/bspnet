pub struct Packer {
    dimensions: [usize; 2],
    skyline: Vec<usize>,
}

impl Packer {
    /// Create an empty skyline within fixed atlas dimensions.
    pub fn create(dimensions: [usize; 2]) -> Self {
        Self {
            dimensions,
            skyline: vec![0; dimensions[0]],
        }
    }

    /// Reserve a rectangle at the lowest fitting position, breaking ties left to right.
    /// Returns None without changing the skyline for empty or unplaceable rectangles.
    pub fn pack(&mut self, dimensions: [usize; 2]) -> Option<[usize; 2]> {
        let [width, height] = dimensions;
        if width == 0 || height == 0 || width > self.dimensions[0] || height > self.dimensions[1] {
            return None;
        }
        let max_y = self.dimensions[1] - height;
        let (x, y) = self
            .skyline
            .windows(width)
            .enumerate()
            .map(|(x, columns)| (x, *columns.iter().max().unwrap()))
            .filter(|&(_, y)| y <= max_y)
            .min_by_key(|&(x, y)| (y, x))?;
        self.skyline[x..x + width].fill(y + height);
        Some([x, y])
    }
}
