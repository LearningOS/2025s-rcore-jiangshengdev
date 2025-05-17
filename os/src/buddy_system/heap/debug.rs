use core::fmt;

impl<const ORDER: usize> fmt::Debug for super::Heap<ORDER> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Heap {{ user: {}, allocated: {}, total: {}",
            self.user, self.allocated, self.total
        )?;
        for (i, list) in self.free_list.iter().enumerate() {
            write!(f, ",\n  free_list[{}]: [", i)?;
            let mut first = true;
            for node in list.iter() {
                if !first {
                    write!(f, ", ")?;
                }
                write!(f, "0x{:x}", node as usize)?;
                first = false;
            }
            write!(f, "]")?;
        }
        write!(f, " }}")
    }
}
