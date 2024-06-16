/// Used as a fast hashset.
pub struct Bitmap {
    bits: Vec<u64>,
}

impl Bitmap {
    pub fn new(size: usize) -> Self {
        let num_elements = (size + 63) / 64; // Number of u64 elements needed
        Bitmap {
            bits: vec![0; num_elements],
        }
    }

    pub fn insert(&mut self, value: usize) {
        let (index, bit) = (value / 64, value % 64);
        if index < self.bits.len() {
            self.bits[index] |= 1 << bit;
        }
    }

    pub fn contains(&self, value: &usize) -> bool {
        let (index, bit) = (value / 64, value % 64);
        if index < self.bits.len() {
            (self.bits[index] & (1 << bit)) != 0
        } else {
            false
        }
    }
}
