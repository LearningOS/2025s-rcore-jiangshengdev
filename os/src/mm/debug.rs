//! Debug and print utilities for memory management
//! 用于内存管理的调试和打印工具

use super::{PageTable, PhysPageNum, VPNRange, VirtAddr, VirtPageNum};
use crate::println_color;
use alloc::format;
use alloc::string::String;
use core::fmt::Write;

/// 打印VPN到PPN映射的配置选项

pub struct MappingPrintOptions {
    /// 是否压缩显示连续映射
    pub compress_identical: bool,
    /// 打印的颜色代码
    pub color_code: u8,
    /// 页数限制（用于大内存区域）
    pub page_limit: Option<usize>,
    /// 每行显示的映射数量
    pub items_per_line: usize,
}

impl MappingPrintOptions {
    /// 创建新的打印配置，使用指定的颜色

    pub fn new(color_code: u8) -> Self {

        Self {
            compress_identical: true,
            color_code,
            page_limit: None,
            items_per_line: 4,
        }
    }

    /// 设置页数限制

    pub fn with_page_limit(mut self, limit: usize) -> Self {

        self.page_limit = Some(limit);

        self
    }
}

/// 单个页面的映射信息

struct PageMapping {
    vpn: VirtPageNum,
    ppn: PhysPageNum,
}

/// 打印单行映射信息，自动处理缓冲和换行

struct MappingPrinter {
    color_code: u8,
    items_per_line: usize,
    line_buffer: String,
    count: usize,
}

impl MappingPrinter {
    fn new(color_code: u8, items_per_line: usize) -> Self {

        Self {
            color_code,
            items_per_line,
            line_buffer: String::new(),
            count: 0,
        }
    }

    fn add_mapping(&mut self, mapping: PageMapping) {

        // 在同一行使用tab格式化输出
        if self.count > 0 && self.count % self.items_per_line == 0 {

            self.flush();
        }

        let mut s = String::new();

        // 格式化输出VPN和PPN
        write!(s, "{:?}->{:?}\t", mapping.vpn, mapping.ppn).unwrap();

        self.line_buffer.push_str(&s);

        self.count += 1;
    }

    fn flush(&mut self) {

        if !self.line_buffer.is_empty() {

            println_color!(self.color_code, "{}", self.line_buffer);

            self.line_buffer = String::new();
        }
    }

    fn print_header(&self, header: &str) {

        println_color!(self.color_code, "{}", header);
    }
}

/// 优化打印VPN到PPN的映射关系
///
/// # 参数
///
/// * `page_table` - 页表的引用
/// * `start_vpn` - 起始虚拟页号
/// * `end_vpn` - 结束虚拟页号（不含此页）
/// * `options` - 打印选项
///
/// # 示例
///
/// ```
/// let options = MappingPrintOptions::new(color::CYAN).with_page_limit(20);
/// print_page_mappings(&memory_set.page_table, start_vpn, end_vpn, options);
/// ```

pub fn print_page_mappings(
    page_table: &PageTable,
    start_vpn: VirtPageNum,
    end_vpn: VirtPageNum,
    options: MappingPrintOptions,
) {

    let total_pages = end_vpn.0 - start_vpn.0;

    let MappingPrintOptions {
        compress_identical,
        color_code,
        page_limit,
        items_per_line,
    } = options;

    // 如果有页数限制且页数过多，使用限制版本
    if let Some(limit) = page_limit {

        if total_pages > limit * 2 {

            print_limited_mappings(
                page_table,
                start_vpn,
                end_vpn,
                color_code,
                limit,
                items_per_line,
            );

            return;
        }
    }

    // 如果页数较少直接全部打印
    if !compress_identical || total_pages <= items_per_line * 3 {

        print_mappings_range(
            page_table,
            start_vpn,
            end_vpn,
            color_code,
            items_per_line,
            None,
        );

        return;
    }

    // 检查是否为连续的相同映射（如Identical映射）
    if check_identical_mapping(page_table, start_vpn, end_vpn) {

        print_identical_mapping_summary(page_table, start_vpn, end_vpn, color_code);
    } else {

        // 非Identical映射，打印开头和结尾
        print_head_tail_mappings(page_table, start_vpn, end_vpn, color_code, items_per_line);
    }
}

/// 检查是否为连续的相同（Identical）映射

fn check_identical_mapping(
    page_table: &PageTable,
    start_vpn: VirtPageNum,
    end_vpn: VirtPageNum,
) -> bool {

    let mut is_identical = true;

    let first_vpn = start_vpn;

    let total_pages = end_vpn.0 - start_vpn.0;

    if let Some(pte) = page_table.translate(first_vpn) {

        let first_ppn = pte.ppn();

        // 检查第一个页面，判断是否可能为Identical映射
        if first_ppn.0 != first_vpn.0 {

            is_identical = false;
        } else {

            // 随机取样检查是否为连续的相同偏移映射
            let sample_count = if total_pages > 10 { 5 } else { 2 };

            let step = total_pages / sample_count;

            for i in 1..sample_count {

                let sample_vpn = VirtPageNum(start_vpn.0 + i * step);

                if let Some(pte) = page_table.translate(sample_vpn) {

                    if pte.ppn().0 != sample_vpn.0 {

                        is_identical = false;

                        break;
                    }
                } else {

                    is_identical = false;

                    break;
                }
            }
        }
    }

    is_identical
}

/// 打印连续相同映射的摘要信息

fn print_identical_mapping_summary(
    _page_table: &PageTable,
    start_vpn: VirtPageNum,
    end_vpn: VirtPageNum,
    color_code: u8,
) {

    let total_pages = end_vpn.0 - start_vpn.0;

    // 连续的Identical映射，只打印起始和结束
    println_color!(
        color_code,
        "Identical mapping: {:?} ~ {:?} -> {:?} ~ {:?} (共 {} 页)",
        start_vpn,
        VirtPageNum(end_vpn.0 - 1),
        PhysPageNum(start_vpn.0),
        PhysPageNum(end_vpn.0 - 1),
        total_pages
    );
}

/// 打印指定范围内的所有映射

fn print_mappings_range(
    page_table: &PageTable,
    start_vpn: VirtPageNum,
    end_vpn: VirtPageNum,
    color_code: u8,
    items_per_line: usize,
    header: Option<&str>,
) {

    let mut printer = MappingPrinter::new(color_code, items_per_line);

    // 如果提供了标题，则打印
    if let Some(h) = header {

        printer.print_header(h);
    }

    // 遍历区间内的所有页面映射
    for vpn in VPNRange::new(start_vpn, end_vpn) {

        // 获取页表项，如果存在
        if let Some(pte) = page_table.translate(vpn) {

            // 确保是有效的映射，无效的PPN可能是尚未完成的映射或边界重叠
            if !pte.is_valid() || pte.ppn().0 == 0 {

                // 对于无效页面，可以尝试在其他内存段查找
                continue;
            }

            let mapping = PageMapping {
                vpn,
                ppn: pte.ppn(),
            };

            printer.add_mapping(mapping);
        }
    }

    // 确保所有映射都被打印出来
    printer.flush();
}

/// 打印开头和结尾的映射，中间使用省略号

fn print_head_tail_mappings(
    page_table: &PageTable,
    start_vpn: VirtPageNum,
    end_vpn: VirtPageNum,
    color_code: u8,
    items_per_line: usize,
) {

    let total_pages = end_vpn.0 - start_vpn.0;

    let head_pages = items_per_line * 2;

    // 打印头部
    print_mappings_range(
        page_table,
        start_vpn,
        VirtPageNum(start_vpn.0 + head_pages.min(total_pages)),
        color_code,
        items_per_line,
        Some("Head mappings:"),
    );

    // 如果页数较多，打印中间省略号和尾部
    if total_pages > head_pages * 2 {

        println_color!(
            color_code,
            "... (省略 {} 页) ...",
            total_pages - head_pages * 2
        );

        // 打印尾部
        print_mappings_range(
            page_table,
            VirtPageNum(end_vpn.0 - head_pages),
            end_vpn,
            color_code,
            items_per_line,
            Some("Tail mappings:"),
        );
    }
}

/// 打印限定数量的映射，适用于非常大的内存区域

fn print_limited_mappings(
    page_table: &PageTable,
    start_vpn: VirtPageNum,
    end_vpn: VirtPageNum,
    color_code: u8,
    limit: usize,
    items_per_line: usize,
) {

    let total_pages = end_vpn.0 - start_vpn.0;

    // 检查是否为连续的Identical映射
    if check_identical_mapping(page_table, start_vpn, end_vpn) {

        print_identical_mapping_summary(page_table, start_vpn, end_vpn, color_code);

        return;
    }

    // 打印头部limit个页
    let head_limit = limit.min(total_pages);

    print_mappings_range(
        page_table,
        start_vpn,
        VirtPageNum(start_vpn.0 + head_limit),
        color_code,
        items_per_line,
        Some(&format!("Head mappings (限制为 {} 页):", limit)),
    );

    if total_pages > head_limit {

        println_color!(
            color_code,
            "... (省略剩余 {} 页) ...",
            total_pages - head_limit
        );

        // 打印一个结尾的页作为样本
        let last_vpn = VirtPageNum(end_vpn.0 - 1);

        if let Some(pte) = page_table.translate(last_vpn) {

            println_color!(color_code, "尾部示例: {:?}->{:?}", last_vpn, pte.ppn());
        }
    }
}

/// 打印内存区域的映射，提供友好的名称和颜色

pub fn print_area_mapping(
    name: &str,
    page_table: &PageTable,
    start_addr: usize,
    end_addr: usize,
    color_code: u8,
    limit: Option<usize>,
) {

    println_color!(color_code, "{} section mapping:", name);

    let start_vpn = VirtAddr::from(start_addr).floor();

    let end_vpn = VirtAddr::from(end_addr).ceil();

    // 不再对边界页进行特殊处理，而是在打印函数内部处理可能的无效PPN
    let options = if let Some(limit) = limit {

        MappingPrintOptions::new(color_code).with_page_limit(limit)
    } else {

        MappingPrintOptions::new(color_code)
    };

    print_page_mappings(page_table, start_vpn, end_vpn, options);
}
