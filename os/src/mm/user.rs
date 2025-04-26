use crate::mm;

#[allow(unused)]
/// 读取用户空间中的结构体 T
pub fn read_user_struct<T: Copy>(token: usize, ptr: *const T) -> T {
    let size = core::mem::size_of::<T>();
    let slices = mm::translated_byte_buffer(token, ptr as *const u8, size);
    let mut result = core::mem::MaybeUninit::<T>::uninit();
    let dst_bytes =
        unsafe { core::slice::from_raw_parts_mut(result.as_mut_ptr() as *mut u8, size) };
    let mut copied = 0;
    for chunk in slices {
        let remaining = size - copied;
        let len = chunk.len().min(remaining);
        dst_bytes[copied..copied + len].copy_from_slice(&chunk[..len]);
        copied += len;
        if copied >= size {
            break;
        }
    }
    unsafe { result.assume_init() }
}

/// 将结构体 T 写入用户空间
pub fn write_user_struct<T: Copy>(token: usize, ptr: *mut T, value: T) {
    let size = core::mem::size_of::<T>();
    let slices = mm::translated_byte_buffer(token, ptr as *const u8, size);
    let src_bytes = unsafe { core::slice::from_raw_parts(&value as *const _ as *const u8, size) };
    let mut copied = 0;
    for chunk in slices {
        let remaining = size - copied;
        let len = chunk.len().min(remaining);
        chunk[..len].copy_from_slice(&src_bytes[copied..copied + len]);
        copied += len;
        if copied >= size {
            break;
        }
    }
}
