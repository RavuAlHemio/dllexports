#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RingBuffer<T: Copy> {
    buffer: Box<[T]>,
    position: usize,
}
impl<T: Copy> RingBuffer<T> {
    pub fn new(initial_value: T, size: usize) -> Self {
        let buffer = vec![initial_value; size].into_boxed_slice();
        Self {
            buffer,
            position: 0,
        }
    }

    pub fn as_slice(&self) -> &[T] { &self.buffer }
    pub fn position(&self) -> usize { self.position }
    pub fn len(&self) -> usize { self.buffer.len() }

    pub fn set_position(&mut self, new_position: usize) {
        if new_position >= self.buffer.len() {
            panic!("new position {} >= size {}", new_position, self.buffer.len());
        }
    }

    pub fn set_at(&mut self, position: usize, value: T) {
        if position >= self.buffer.len() {
            panic!("position {} >= size {}", position, self.buffer.len());
        }
        self.buffer[position] = value;
    }

    pub fn push(&mut self, value: T) {
        self.buffer[self.position] = value;
        self.position = (self.position + 1) % self.buffer.len();
    }

    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iterable: I) {
        for item in iterable {
            self.push(item);
        }
    }

    pub fn recall(&mut self, lookback: usize, length: usize) -> Vec<T> {
        let mut ret = Vec::with_capacity(length);
        let mut index = if lookback > self.position {
            // we have to wrap around at 0
            self.buffer.len() - (lookback - self.position)
        } else {
            self.position - lookback
        };
        for _ in 0..length {
            let b = self.buffer[index];
            ret.push(b);
            self.push(b);
            index = (index + 1) % self.buffer.len();
        }
        ret
    }
}
