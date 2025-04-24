# 通用变量/宏提取指南

本文档介绍如何将函数或宏内部的变量、逻辑提取到独立函数或宏中，以提高代码可读性和可维护性。适用于 Rust 项目中的
`lazy_static!`、常量、变量抽取等场景。

---

## 1. 提取流程概述

1. 确定待提取的逻辑或变量。
2. 在文件合适位置（通常靠近使用处或按模块组织）定义新的独立函数或宏，并为其编写清晰的文档注释。
3. 将原位置的实现替换为对新函数或宏的调用。
4. 更新或补充注释，描述新函数/宏的功能，而非实现细节。
5. 运行编译和测试，确保无错误。

---

## 2. 前后示例

### 示例 1：`lazy_static!` 中初始化逻辑

#### 变更前

```rust
lazy_static! {
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(
        TaskControlBlock::new(get_app_data_by_name("ch5b_initproc").unwrap())
    );
}
```

#### 变更后

```rust
/// 创建并返回初始进程的 TaskControlBlock 实例。
fn create_initproc() -> Arc<TaskControlBlock> {
    Arc::new(TaskControlBlock::new(
        get_app_data_by_name("ch5b_initproc").unwrap()
    ))
}

lazy_static! {
    pub static ref INITPROC: Arc<TaskControlBlock> = create_initproc();
}
```

---

### 示例 2：一般函数内部变量提取

#### 变更前

```rust
fn process_data(data: &[u8]) {
    let hash = sha256::digest(data);
    // 使用 hash 执行后续逻辑
}
```

#### 变更后

```rust
/// 计算并返回数据的 SHA256 哈希值。
fn compute_data_hash(data: &[u8]) -> HashValue {
    sha256::digest(data)
}

fn process_data(data: &[u8]) {
    let hash = compute_data_hash(data);
    // 使用 hash 执行后续逻辑
}
```

---

### 示例 3：提取 `lazy_static!` 中内核空间初始化逻辑

#### 变更前

```rust
lazy_static! {
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> =
        Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) });
}
```

#### 变更后

```rust
/// 创建并返回内核初始内存映射的 UPSafeCell 实例。
fn create_kernel_space() -> Arc<UPSafeCell<MemorySet>> {
    Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) })
}

lazy_static! {
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> = create_kernel_space();
}
```

### 示例 4：提取 `lazy_static!` 中处理器管理实例初始化

#### 变更前

```rust
lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> =
        unsafe { UPSafeCell::new(Processor::new()) };
}
```

#### 变更后

```rust
/// 创建并返回 Processor 的 UPSafeCell 实例。
fn create_processor() -> UPSafeCell<Processor> {
    unsafe { UPSafeCell::new(Processor::new()) }
}

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = create_processor();
}
```

---

## 3. 通用注意事项

- 函数/宏名应反映其核心功能，并遵循项目命名规范。
- 文档注释应说明输入输出、边界条件和用途。
- 提取后务必编译测试，确保功能等价。
- 在大型项目中，可将此类最佳实践纳入贡献指南，便于团队统一。

---

> 以上即为通用的变量/宏提取流程示例。后续如有其他场景需求，可在此基础上进行扩展。
