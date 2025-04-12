//! 单处理器内部可变性原语

use core::cell::{RefCell, RefMut};

/// 将静态数据结构包装在其中，使我们能够
/// 在不使用任何 `unsafe` 的情况下访问它。
///
/// 我们应该只在单处理器环境中使用它。
///
/// 为了获取内部数据的可变引用，调用
/// `exclusive_access`。

pub struct UPSafeCell<T> {
    /// 内部数据
    inner: RefCell<T>,
}

unsafe impl<T> Sync for UPSafeCell<T> {}

impl<T> UPSafeCell<T> {
    /// # Safety
    /// # 安全性
    /// 用户负责保证内部结构只在单处理器环境中使用。

    pub unsafe fn new(value: T) -> Self {

        Self {
            inner: RefCell::new(value),
        }
    }

    /// 如果数据已被借用则触发 panic。

    pub fn exclusive_access(&self) -> RefMut<'_, T> {

        self.inner.borrow_mut()
    }
}
