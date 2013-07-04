

pub struct DirectWireValue<T> {
    value : T
}

impl<T : Copy> DirectWireValue<T> {

    #[inline(always)]
    pub fn get(&self) -> T { copy self.value }

    #[inline(always)]
    pub fn set(&mut self, value : T) { self.value = value }
}

pub type WireValue<T> = DirectWireValue<T>;

// TODO handle big endian systems.