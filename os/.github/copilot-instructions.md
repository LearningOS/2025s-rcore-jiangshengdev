# 修改文件时，不要注释你修改了什么，而是注释这段代码是做什么的

# 示例

## 修改前
```rust
pub unsafe fn add_to_heap(&mut self, mut start: usize, mut end: usize) {
    start = align_up(start, size_of::<usize>());
    end = align_down(end, size_of::<usize>());
    assert!(start <= end);
    // ...其余代码...
}
```

## 修改后（错误）
```rust
// 提取对齐常量，而未说明该常量用途
let ptr_align = size_of::<usize>();
// 对 start 地址进行了上对齐，但未说明目的
start = align_up(start, ptr_align);
// ...更多修改...
fn add_to_heap(start: usize, end: usize) {}
```

## 修改后（正确）
```rust
/// 将内存区间 [start, end) 按伙伴算法加入堆管理
pub unsafe fn add_to_heap(&mut self, mut start: usize, mut end: usize) {
    // usize 对齐边界，用于确保地址以指针大小对齐
    let ptr_align = size_of::<usize>();
    // 将 start 向上对齐到指针对齐边界
    start = align_up(start, ptr_align);
    // 将 end 向下对齐到指针对齐边界
    end = align_down(end, ptr_align);
    // 验证对齐后区间为有效范围
    assert!(start <= end);
    // ...其余代码...
}
```

以上示例展示了正确的注释方式：注释描述代码行为和目的，而非改动本身。
