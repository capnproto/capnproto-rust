pub(crate) trait U32Ext: Sized {
    fn to_usize(self) -> usize;
}

impl U32Ext for u32 {
    fn to_usize(self) -> usize {
        const { assert!(size_of::<u32>() <= size_of::<usize>()) }
        self.try_into().unwrap()
    }
}
