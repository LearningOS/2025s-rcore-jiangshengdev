//! Debug and print utilities for memory management
//! 用于内存管理的调试和打印工具

use super::{PTEFlags, PageTable, PhysPageNum, VPNRange, VirtAddr, VirtPageNum};
use crate::println_color;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

/// 单个页面的映射信息

#[derive(Clone)]

struct PageMapping {
    vpn: VirtPageNum,
    ppn: PhysPageNum,
    flags: PTEFlags,
}

/// 缓冲映射打印器：收集映射信息，并在刷新时统一输出

struct MappingPrinter {
    color_code: u8,
    line_buffer: String,
    count: usize,
}

impl MappingPrinter {
    fn new(color_code: u8) -> Self {

        Self {
            color_code,
            line_buffer: String::new(),
            count: 0,
        }
    }

    fn add_mapping(&mut self, mapping: PageMapping) {

        let mut s = String::new();

        write!(s, "VPN:{:#x} -> PPN:{:#x}\t", mapping.vpn.0, mapping.ppn.0).unwrap();

        self.line_buffer.push_str(&s);

        self.count += 1;
    }

    fn flush(&mut self) {

        if !self.line_buffer.is_empty() {

            println_color!(self.color_code, "{}", self.line_buffer);

            self.line_buffer.clear();
        }
    }

    fn print_header(&self, header: &str) {

        println_color!(self.color_code, "{}", header);
    }
}

// 辅助函数：将 PTEFlags 格式化为以 " | " 分隔的权限字符串
fn format_flags(flags: PTEFlags) -> String {

    let mut parts = Vec::new();

    if flags.contains(PTEFlags::R) {

        parts.push("R");
    }

    if flags.contains(PTEFlags::W) {

        parts.push("W");
    }

    if flags.contains(PTEFlags::X) {

        parts.push("X");
    }

    if flags.contains(PTEFlags::U) {

        parts.push("U");
    }

    parts.join(" | ")
}

/// 打印所有页面映射信息，采用压缩输出方式

pub fn print_page_mappings(
    page_table: &PageTable,
    start_vpn: VirtPageNum,
    end_vpn: VirtPageNum,
    color_code: u8,
) {

    print_mappings_range(page_table, start_vpn, end_vpn, color_code, None, true);
}

/// 打印指定范围内的页面映射，支持分组与压缩输出

fn print_mappings_range(
    page_table: &PageTable,
    start_vpn: VirtPageNum,
    end_vpn: VirtPageNum,
    color_code: u8,
    header: Option<&str>,
    compress: bool,
) {

    let mut printer = MappingPrinter::new(color_code);

    if let Some(h) = header {

        printer.print_header(h);
    }

    fn flush_group(printer: &mut MappingPrinter, group: &mut Vec<PageMapping>, color_code: u8) {

        if group.is_empty() {

            return;
        }

        if group.len() >= 10 {

            let start = group.first().unwrap();

            let end = group.last().unwrap();

            println_color!(
                color_code,
                "VPN:{:#x} -> PPN:{:#x}",
                start.vpn.0,
                start.ppn.0
            );

            println_color!(
                color_code,
                "...\nVPN:{:#x} -> PPN:{:#x}",
                end.vpn.0,
                end.ppn.0
            );

            println_color!(color_code, "{}×", group.len());

            println_color!(color_code, "{}", format_flags(start.flags));
        } else {

            for mapping in group.iter() {

                println_color!(
                    printer.color_code,
                    "VPN:{:#x} -> PPN:{:#x}\t{}",
                    mapping.vpn.0,
                    mapping.ppn.0,
                    format_flags(mapping.flags)
                );
            }
        }

        group.clear();
    }

    if compress {

        let mut group: Vec<PageMapping> = Vec::new();

        for vpn in VPNRange::new(start_vpn, end_vpn) {

            if let Some(pte) = page_table.translate(vpn) {

                if !pte.is_valid() || pte.ppn().0 == 0 {

                    continue;
                }

                let mapping = PageMapping {
                    vpn,
                    ppn: pte.ppn(),
                    flags: pte.flags(),
                };

                if mapping.ppn.0 == mapping.vpn.0 {

                    if let Some(last) = group.last() {

                        if mapping.vpn.0 == last.vpn.0 + 1 && mapping.ppn.0 == last.ppn.0 + 1 {

                            group.push(mapping);
                        } else {

                            flush_group(&mut printer, &mut group, color_code);

                            group.push(mapping);
                        }
                    } else {

                        group.push(mapping);
                    }
                } else {

                    flush_group(&mut printer, &mut group, color_code);

                    printer.add_mapping(mapping);
                }
            }
        }

        flush_group(&mut printer, &mut group, color_code);
    } else {

        for vpn in VPNRange::new(start_vpn, end_vpn) {

            if let Some(pte) = page_table.translate(vpn) {

                if !pte.is_valid() || pte.ppn().0 == 0 {

                    continue;
                }

                let mapping = PageMapping {
                    vpn,
                    ppn: pte.ppn(),
                    flags: pte.flags(),
                };

                printer.add_mapping(mapping);
            }
        }
    }

    printer.flush();
}

/// 打印内存区域映射信息，提供友好名称与颜色

pub fn print_area_mapping(
    name: &str,
    page_table: &PageTable,
    start_addr: usize,
    end_addr: usize,
    color_code: u8,
) {

    println_color!(color_code, "{} section mapping:", name);

    let start_vpn = VirtAddr::from(start_addr).floor();

    let end_vpn = VirtAddr::from(end_addr).ceil();

    print_page_mappings(page_table, start_vpn, end_vpn, color_code);
}

/// 打印单个页面的映射信息

pub fn print_mapped_page(page_table: &PageTable, va: VirtAddr, name: &str, color_code: u8) {

    let floor_va = va.floor();

    let pte = page_table.translate(floor_va).unwrap();

    println_color!(
        color_code,
        "{} mapped:\n{:?} -> {:?}",
        name,
        floor_va,
        pte.ppn()
    );
}
