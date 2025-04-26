use crate::mm;
use crate::mm::{PageTable, VirtAddr};

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

fn do_user_memory(token: usize, addr: usize, data: Option<u8>) -> isize {
    let va = VirtAddr::from(addr);
    let vpn = va.floor();
    let Some(pte) = PageTable::from_token(token).translate(vpn) else {
        return -1;
    };
    if !pte.is_user_accessible() {
        return -1;
    }
    match data {
        Some(byte) => {
            // writing
            if !pte.writable() {
                return -1;
            }
            let ppn = pte.ppn();
            ppn.get_bytes_array()[va.page_offset()] = byte;
            0
        }
        None => {
            // reading
            if !pte.readable() {
                return -1;
            }
            let ppn = pte.ppn();
            ppn.get_bytes_array()[va.page_offset()] as isize
        }
    }
}

/// 从用户空间读取一个字节
pub fn read_user_memory(token: usize, addr: usize) -> isize {
    do_user_memory(token, addr, None)
}

/// 向用户空间写入一个字节
pub fn write_user_memory(token: usize, addr: usize, data: usize) -> isize {
    do_user_memory(token, addr, Some(data as u8))
}
