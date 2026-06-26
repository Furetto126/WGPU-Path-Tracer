pub trait MinMax<T> {
    fn set_min(&mut self, other: T);
    fn set_max(&mut self, other: T);
} 

impl<const N: usize> MinMax<[f32; N]> for [f32; N] {
    fn set_min(&mut self, other: [f32; N]) {
        for i in 0..N {
            self[i] = self[i].min(other[i]);
        }
    }

    fn set_max(&mut self, other: [f32; N]) {
        for i in 0..N {
            self[i] = self[i].max(other[i]);
        }
    }
}