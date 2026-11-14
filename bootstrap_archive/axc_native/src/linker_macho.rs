//! Axon v5.0 Direct Mach-O 64-Bit Executable Linker (`linker_macho.rs`)
//! Packages Apple Silicon `arm64` (16 KB page alignment) or `x86_64` code directly into
//! OS-clean macOS Mach-O binaries with `__PAGEZERO`, strict W^X `__TEXT`, and `LC_FUNCTION_STARTS`.

pub fn build_macho64_arm64(text: &[u8], rodata: &[u8], fn_starts: &[u32]) -> Vec<u8> {
    let page_size = 0x4000usize; // 16 KB Apple Silicon hardware page size!
    let text_file_off = 0x0400usize; // Start `__text` at offset 1024 inside `__TEXT` segment
    let text_end = text_file_off + text.len();
    let const_off = (text_end + 15) & !15;
    let const_end = const_off + rodata.len();

    // Encode `LC_FUNCTION_STARTS` ULEB128 delta table
    let mut fstarts_blob = Vec::new();
    let mut prev = 0u32;
    for (i, &off) in fn_starts.iter().enumerate() {
        let abs_off = text_file_off as u32 + off;
        let delta = if i == 0 { abs_off } else { abs_off.saturating_sub(prev) };
        prev = abs_off;
        push_uleb128(&mut fstarts_blob, delta);
    }
    fstarts_blob.push(0);
    while fstarts_blob.len() % 8 != 0 {
        fstarts_blob.push(0);
    }

    let linkedit_off = ((const_end + page_size - 1) / page_size) * page_size;
    let linkedit_size = ((fstarts_blob.len() + page_size - 1) / page_size) * page_size;
    let total_size = linkedit_off + fstarts_blob.len();

    let mut mo = vec![0u8; total_size.max(page_size)];

    // Load commands:
    // 1. LC_SEGMENT_64 `__PAGEZERO` (72 bytes)
    // 2. LC_SEGMENT_64 `__TEXT` with 2 sections (`__text`, `__const`) (72 + 2*80 = 232 bytes)
    // 3. LC_SEGMENT_64 `__LINKEDIT` (72 bytes)
    // 4. LC_FUNCTION_STARTS (16 bytes)
    // 5. LC_UNIXTHREAD / entry (184 bytes -> or LC_MAIN 24 bytes!)
    let ncmds = 5u32;
    let sizeofcmds = 72 + 232 + 72 + 16 + 24;

    // === mach_header_64 (32 bytes) ===
    let mut p = 0usize;
    put32(&mut mo, &mut p, 0xFEED_FACF); // MH_MAGIC_64
    put32(&mut mo, &mut p, 0x0100_000C); // CPU_TYPE_ARM64
    put32(&mut mo, &mut p, 0x0000_0000); // CPU_SUBTYPE_ARM64_ALL
    put32(&mut mo, &mut p, 2);           // MH_EXECUTE
    put32(&mut mo, &mut p, ncmds);
    put32(&mut mo, &mut p, sizeofcmds);
    put32(&mut mo, &mut p, 0x0020_0085); // MH_NOUNDEFS | MH_DYLDLINK | MH_TWOLEVEL | MH_PIE
    put32(&mut mo, &mut p, 0);           // reserved

    let text_vmbase: u64 = 0x0001_0000_0000;

    // 1. LC_SEGMENT_64 (0x19) `__PAGEZERO`
    write_seg64(&mut mo, &mut p, b"__PAGEZERO\0\0\0\0\0\0", 0, text_vmbase, 0, 0, 0, 0, 0);

    // 2. LC_SEGMENT_64 `__TEXT` (VM_PROT_READ | VM_PROT_EXECUTE = 5, Strict W^X!)
    put32(&mut mo, &mut p, 0x19); // LC_SEGMENT_64
    put32(&mut mo, &mut p, 232);  // cmdsize = 72 + 2*80
    mo[p..p + 16].copy_from_slice(b"__TEXT\0\0\0\0\0\0\0\0\0\0");
    p += 16;
    put64(&mut mo, &mut p, text_vmbase);
    put64(&mut mo, &mut p, linkedit_off as u64);
    put64(&mut mo, &mut p, 0);
    put64(&mut mo, &mut p, linkedit_off as u64);
    put32(&mut mo, &mut p, 5); // maxprot = R | X
    put32(&mut mo, &mut p, 5); // initprot = R | X
    put32(&mut mo, &mut p, 2); // nsects = 2
    put32(&mut mo, &mut p, 0);

    // Section 1: `__text`
    write_sect64(
        &mut mo,
        &mut p,
        b"__text\0\0\0\0\0\0\0\0\0\0",
        b"__TEXT\0\0\0\0\0\0\0\0\0\0",
        text_vmbase + text_file_off as u64,
        text.len() as u64,
        text_file_off as u32,
        2,
        0x8000_0400, // S_ATTR_PURE_INSTRUCTIONS | S_ATTR_SOME_INSTRUCTIONS
    );
    // Section 2: `__const`
    write_sect64(
        &mut mo,
        &mut p,
        b"__const\0\0\0\0\0\0\0\0\0",
        b"__TEXT\0\0\0\0\0\0\0\0\0\0",
        text_vmbase + const_off as u64,
        rodata.len() as u64,
        const_off as u32,
        4,
        0,
    );

    // 3. LC_SEGMENT_64 `__LINKEDIT` (VM_PROT_READ = 1)
    write_seg64(
        &mut mo,
        &mut p,
        b"__LINKEDIT\0\0\0\0\0\0",
        text_vmbase + linkedit_off as u64,
        linkedit_size as u64,
        linkedit_off as u64,
        fstarts_blob.len() as u64,
        1,
        1,
        0,
    );

    // 4. LC_FUNCTION_STARTS (0x26)
    put32(&mut mo, &mut p, 0x26);
    put32(&mut mo, &mut p, 16);
    put32(&mut mo, &mut p, linkedit_off as u32);
    put32(&mut mo, &mut p, fstarts_blob.len() as u32);

    // 5. LC_MAIN (0x80000028)
    put32(&mut mo, &mut p, 0x8000_0028);
    put32(&mut mo, &mut p, 24);
    put64(&mut mo, &mut p, text_file_off as u64); // entryoff
    put64(&mut mo, &mut p, 0);                    // stacksize
    debug_assert_eq!(p, 32 + sizeofcmds as usize);

    mo[text_file_off..text_file_off + text.len()].copy_from_slice(text);
    if !rodata.is_empty() {
        mo[const_off..const_off + rodata.len()].copy_from_slice(rodata);
    }
    mo[linkedit_off..linkedit_off + fstarts_blob.len()].copy_from_slice(&fstarts_blob);
    mo
}

fn push_uleb128(out: &mut Vec<u8>, mut val: u32) {
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if val == 0 {
            break;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn write_seg64(
    mo: &mut [u8],
    p: &mut usize,
    segname: &[u8; 16],
    vmaddr: u64,
    vmsize: u64,
    fileoff: u64,
    filesize: u64,
    maxprot: u32,
    initprot: u32,
    nsects: u32,
) {
    put32(mo, p, 0x19); // LC_SEGMENT_64
    put32(mo, p, 72);
    mo[*p..*p + 16].copy_from_slice(segname);
    *p += 16;
    put64(mo, p, vmaddr);
    put64(mo, p, vmsize);
    put64(mo, p, fileoff);
    put64(mo, p, filesize);
    put32(mo, p, maxprot);
    put32(mo, p, initprot);
    put32(mo, p, nsects);
    put32(mo, p, 0);
}

#[allow(clippy::too_many_arguments)]
fn write_sect64(
    mo: &mut [u8],
    p: &mut usize,
    sectname: &[u8; 16],
    segname: &[u8; 16],
    addr: u64,
    size: u64,
    offset: u32,
    align_pow2: u32,
    flags: u32,
) {
    mo[*p..*p + 16].copy_from_slice(sectname);
    *p += 16;
    mo[*p..*p + 16].copy_from_slice(segname);
    *p += 16;
    put64(mo, p, addr);
    put64(mo, p, size);
    put32(mo, p, offset);
    put32(mo, p, align_pow2);
    put32(mo, p, 0); // reloff
    put32(mo, p, 0); // nreloc
    put32(mo, p, flags);
    put32(mo, p, 0); // reserved1
    put32(mo, p, 0); // reserved2
    put32(mo, p, 0); // reserved3
}

fn put32(b: &mut [u8], p: &mut usize, v: u32) {
    b[*p..*p + 4].copy_from_slice(&v.to_le_bytes());
    *p += 4;
}
fn put64(b: &mut [u8], p: &mut usize, v: u64) {
    b[*p..*p + 8].copy_from_slice(&v.to_le_bytes());
    *p += 8;
}
