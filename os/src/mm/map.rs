use crate::mm::MapPermission;

bitflags! {
    /// prot flags
    pub struct ProtFlags: u8 {
        /// Readable
        const R = 1 << 0;
        /// Writable
        const W = 1 << 1;
        /// Executable
        const X = 1 << 2;
    }
}

/// 解析标志位，若有非法标志则返回 None
pub fn parse_prot_flags(prot: usize) -> Option<ProtFlags> {
    ProtFlags::from_bits(prot as u8).filter(|flags| ProtFlags::all().contains(*flags))
}

// 从 ProtFlags 到 MapPermission 的转换实现，默认添加 U 权限
impl From<ProtFlags> for MapPermission {
    fn from(flags: ProtFlags) -> Self {
        let mut permission = MapPermission::U; // 默认添加 U 权限

        if flags.contains(ProtFlags::R) {
            permission |= MapPermission::R;
        }
        if flags.contains(ProtFlags::W) {
            permission |= MapPermission::W;
        }
        if flags.contains(ProtFlags::X) {
            permission |= MapPermission::X;
        }

        permission
    }
}
