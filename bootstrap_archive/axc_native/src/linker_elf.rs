//! Axon v5.0 Direct ELF64 Executable Linker (`linker_elf.rs`)
//! Packages native `x86_64` or `aarch64` machine code directly into OS-clean Linux ELF64 binaries:
//! - Strict W^X Program Headers (`PT_LOAD` RX `.text`, `PT_LOAD` R `.rodata` + `.eh_frame`, `PT_GNU_STACK` RW non-exec)
//! - Genuine DWARF CFI `.eh_frame` CIE + FDE unwind records + `PT_GNU_EH_FRAME`

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfArch {
    X86_64,
    Aarch64,
}

pub fn build_elf64_executable(arch: ElfArch, text: &[u8], rodata: &[u8]) -> Vec<u8> {
    let base_vaddr: u64 = 0x0040_0000;
    let ehdr_size = 64usize;
    let phdr_count = 4usize; // PT_LOAD (RX), PT_LOAD (R), PT_GNU_EH_FRAME, PT_GNU_STACK
    let phdr_size = phdr_count * 56;
    let text_file_off = 0x1000usize; // Page-aligned `.text` for clean W^X mapping!
    let text_vaddr = base_vaddr + text_file_off as u64;

    // Synthesize DWARF `.eh_frame` CIE + FDE covering `.text`
    let eh_frame = build_dwarf_eh_frame(text.len() as u32, arch);

    let ro_file_off = (text_file_off + text.len() + 0xFFF) & !0xFFF;
    let ro_vaddr = base_vaddr + ro_file_off as u64;
    let eh_vaddr = ro_vaddr + rodata.len() as u64;
    let ro_total_len = rodata.len() + eh_frame.len();

    let total_size = ro_file_off + ro_total_len;
    let mut elf = vec![0u8; total_size];

    // === ELF64 Header (64 bytes) ===
    elf[0..4].copy_from_slice(b"\x7FELF");
    elf[4] = 2; // EI_CLASS = ELFCLASS64
    elf[5] = 1; // EI_DATA = ELFDATA2LSB
    elf[6] = 1; // EI_VERSION = EV_CURRENT
    elf[7] = 0; // EI_OSABI = SYSV

    let mut p = 16usize;
    put16(&mut elf, &mut p, 2); // e_type = ET_EXEC
    put16(
        &mut elf,
        &mut p,
        match arch {
            ElfArch::X86_64 => 62,   // EM_X86_64
            ElfArch::Aarch64 => 183, // EM_AARCH64
        },
    );
    put32(&mut elf, &mut p, 1); // e_version
    put64(&mut elf, &mut p, text_vaddr); // e_entry
    put64(&mut elf, &mut p, ehdr_size as u64); // e_phoff = 64
    put64(&mut elf, &mut p, 0); // e_shoff
    put32(&mut elf, &mut p, 0); // e_flags
    put16(&mut elf, &mut p, ehdr_size as u16); // e_ehsize = 64
    put16(&mut elf, &mut p, 56); // e_phentsize = 56
    put16(&mut elf, &mut p, phdr_count as u16); // e_phnum
    put16(&mut elf, &mut p, 64); // e_shentsize
    put16(&mut elf, &mut p, 0);  // e_shnum
    put16(&mut elf, &mut p, 0);  // e_shstrndx
    debug_assert_eq!(p, ehdr_size);

    // === Program Headers (4 * 56 = 224 bytes) ===
    // 1. PT_LOAD (1): `.text` — PF_R (4) | PF_X (1) = 5 (Strict W^X: NO PF_W!)
    write_phdr(
        &mut elf,
        &mut p,
        1,
        5,
        text_file_off as u64,
        text_vaddr,
        text.len() as u64,
        text.len() as u64,
        0x1000,
    );
    // 2. PT_LOAD (1): `.rodata` + `.eh_frame` — PF_R (4)
    write_phdr(
        &mut elf,
        &mut p,
        1,
        4,
        ro_file_off as u64,
        ro_vaddr,
        ro_total_len as u64,
        ro_total_len as u64,
        0x1000,
    );
    // 3. PT_GNU_EH_FRAME (0x6474e550): points to `.eh_frame` — PF_R (4)
    write_phdr(
        &mut elf,
        &mut p,
        0x6474_e550,
        4,
        (ro_file_off + rodata.len()) as u64,
        eh_vaddr,
        eh_frame.len() as u64,
        eh_frame.len() as u64,
        4,
    );
    // 4. PT_GNU_STACK (0x6474e551): PF_R (4) | PF_W (2) = 6 (Strict NX stack!)
    write_phdr(&mut elf, &mut p, 0x6474_e551, 6, 0, 0, 0, 0, 16);
    debug_assert_eq!(p, ehdr_size + phdr_size);

    // Copy `.text`, `.rodata`, and `.eh_frame`
    elf[text_file_off..text_file_off + text.len()].copy_from_slice(text);
    elf[ro_file_off..ro_file_off + rodata.len()].copy_from_slice(rodata);
    elf[ro_file_off + rodata.len()..ro_file_off + ro_total_len].copy_from_slice(&eh_frame);

    elf
}

fn build_dwarf_eh_frame(text_len: u32, arch: ElfArch) -> Vec<u8> {
    let mut eh = Vec::with_capacity(48);
    let ret_reg: u8 = match arch {
        ElfArch::X86_64 => 16,  // RIP
        ElfArch::Aarch64 => 30, // X30 (LR)
    };
    // CIE record (16 bytes payload)
    eh.extend_from_slice(&16u32.to_le_bytes()); // Length
    eh.extend_from_slice(&0u32.to_le_bytes());  // CIE ID = 0
    eh.push(1);                                 // Version 1
    eh.extend_from_slice(b"zR\0");              // Augmentation string
    eh.push(1);                                 // Code alignment factor
    eh.push(0x78);                              // Data alignment factor (-8 SLEB128)
    eh.push(ret_reg);                           // Return address register
    eh.push(1);                                 // Augmentation data len = 1
    eh.push(0x1B);                              // DW_EH_PE_pcrel | DW_EH_PE_sdata4
    while (eh.len() - 4) % 8 != 0 {
        eh.push(0); // DW_CFA_nop padding
    }
    let cie_body_len = (eh.len() - 4) as u32;
    eh[0..4].copy_from_slice(&cie_body_len.to_le_bytes());

    // FDE record covering `.text`
    let fde_start = eh.len();
    eh.extend_from_slice(&16u32.to_le_bytes()); // FDE length
    let cie_ptr = (fde_start + 4) as u32;
    eh.extend_from_slice(&cie_ptr.to_le_bytes());
    eh.extend_from_slice(&(-0x1000i32).to_le_bytes()); // PC-begin relative
    eh.extend_from_slice(&text_len.to_le_bytes());     // PC-range
    eh.push(0);                                        // Augmentation data len
    while (eh.len() - fde_start - 4) % 8 != 0 {
        eh.push(0);
    }
    let fde_body_len = (eh.len() - fde_start - 4) as u32;
    eh[fde_start..fde_start + 4].copy_from_slice(&fde_body_len.to_le_bytes());

    // Zero terminator (4 bytes)
    eh.extend_from_slice(&0u32.to_le_bytes());
    eh
}

#[allow(clippy::too_many_arguments)]
fn write_phdr(
    elf: &mut [u8],
    p: &mut usize,
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
) {
    put32(elf, p, p_type);
    put32(elf, p, p_flags);
    put64(elf, p, p_offset);
    put64(elf, p, p_vaddr);
    put64(elf, p, p_vaddr); // p_paddr
    put64(elf, p, p_filesz);
    put64(elf, p, p_memsz);
    put64(elf, p, p_align);
}

fn put16(b: &mut [u8], p: &mut usize, v: u16) {
    b[*p..*p + 2].copy_from_slice(&v.to_le_bytes());
    *p += 2;
}
fn put32(b: &mut [u8], p: &mut usize, v: u32) {
    b[*p..*p + 4].copy_from_slice(&v.to_le_bytes());
    *p += 4;
}
fn put64(b: &mut [u8], p: &mut usize, v: u64) {
    b[*p..*p + 8].copy_from_slice(&v.to_le_bytes());
    *p += 8;
}
