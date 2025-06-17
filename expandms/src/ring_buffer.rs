pub struct RingBuffer<T: Copy, const SIZE: usize> {
    buffer: [T; SIZE],
    position: usize,
}
impl<T: Copy, const SIZE: usize> RingBuffer<T, SIZE> {
    pub fn new(initial_value: T) -> Self {
        let buffer = [initial_value; SIZE];
        Self {
            buffer,
            position: 0,
        }
    }

    pub fn as_slice(&self) -> &[T] { &self.buffer }
    pub fn position(&self) -> usize { self.position }
    pub fn len(&self) -> usize { SIZE }

    pub fn set_position(&mut self, new_position: usize) {
        if new_position >= SIZE {
            panic!("new position {} >= size {}", new_position, SIZE);
        }
    }

    pub fn set_at(&mut self, position: usize, value: T) {
        if position >= SIZE {
            panic!("position {} >= size {}", position, SIZE);
        }
        self.buffer[position] = value;
    }

    pub fn push(&mut self, value: T) {
        self.buffer[self.position] = value;
        self.position = (self.position + 1) % SIZE;
    }

    pub fn extend<I: IntoIterator<Item = T>>(&mut self, iterable: I) {
        for item in iterable {
            self.push(item);
        }
    }

    pub fn recall(&mut self, mut lookback: usize, length: usize) -> Vec<T> {
        let mut ret = Vec::with_capacity(length);
        lookback %= SIZE;
        if lookback > self.position {
            // we have to wrap around at 0
            lookback = SIZE - (lookback - self.position);
        }
        for _ in 0..length {
            let b = self.buffer[lookback];
            ret.push(b);
            self.push(b);
            lookback = (lookback + 1) % SIZE;
        }
        ret
    }
}
