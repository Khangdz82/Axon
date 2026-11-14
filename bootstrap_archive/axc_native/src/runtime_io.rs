//! Axon v5.0 Native OS I/O, Safe Dynamic Buffers & Whitebox Intrinsics (`runtime_io.rs`)
//! Tree-shaken zero-CRT x86_64 helpers talking directly to `KERNEL32.dll` via Win64 ABI.

use crate::pe::*;

pub const HELPER_FS_READ: usize = 100_010;
pub const HELPER_FS_WRITE: usize = 100_011;
pub const HELPER_ARGS_COUNT: usize = 100_012;
pub const HELPER_ARGS_GET: usize = 100_013;
pub const HELPER_STR_EQ: usize = 100_014;

pub trait EmitterIo {
    fn code_mut(&mut self) -> &mut Vec<u8>;
    fn register_helper(&mut self, id: usize, start: usize, end: usize);
    fn call_iat(&mut self, slot: usize);
}

/// Emits tree-shaken Win64 helpers for File I/O, CLI Parsing & String Comparison
pub fn emit_extended_helpers<E: EmitterIo>(em: &mut E) {
    emit_str_eq_helper(em);
    emit_args_helpers(em);
    emit_fs_read_helper(em);
    emit_fs_write_helper(em);
}

fn align16(code: &mut Vec<u8>) {
    while code.len() % 16 != 0 {
        code.push(0xCC);
    }
}

fn emit_jcc8(code: &mut Vec<u8>, op: u8) -> usize {
    code.push(op);
    let pos = code.len();
    code.push(0);
    pos
}

fn patch_jcc8(code: &mut [u8], pos: usize, target: usize) {
    let rel = (target as isize - (pos as isize + 1)) as i8;
    code[pos] = rel as u8;
}

/// `str_eq(rcx = a_ptr, rdx = a_len, r8 = b_ptr, r9 = b_len) -> rax (0 or 1)`
fn emit_str_eq_helper<E: EmitterIo>(em: &mut E) {
    align16(em.code_mut());
    let start = em.code_mut().len();
    let c = em.code_mut();
    c.extend_from_slice(&[0xF3, 0x0F, 0x1E, 0xFA]); // ENDBR64
    c.extend_from_slice(&[0x4C, 0x39, 0xCA]); // CMP RDX, R9
    let jne_len = emit_jcc8(c, 0x75); // JNE .ret_false
    c.extend_from_slice(&[0x31, 0xC0]); // XOR EAX, EAX (idx = 0)
    let loop_pos = c.len();
    c.extend_from_slice(&[0x48, 0x39, 0xD0]); // CMP RAX, RDX
    let jae_true = emit_jcc8(c, 0x73); // JAE .ret_true
    c.extend_from_slice(&[0x44, 0x8A, 0x14, 0x01, 0x45, 0x3A, 0x14, 0x00]); // MOV R10B, [RCX+RAX]; CMP R10B, [R8+RAX]
    let jne_byte = emit_jcc8(c, 0x75); // JNE .ret_false
    c.extend_from_slice(&[0x48, 0xFF, 0xC0]); // INC RAX
    let jmp_loop = emit_jcc8(c, 0xEB);
    patch_jcc8(c, jmp_loop, loop_pos);

    let ret_true = c.len();
    c.extend_from_slice(&[0xB8, 0x01, 0x00, 0x00, 0x00, 0xC3]); // MOV EAX, 1; RET
    let ret_false = c.len();
    c.extend_from_slice(&[0x31, 0xC0, 0xC3]); // XOR EAX, EAX; RET

    patch_jcc8(c, jae_true, ret_true);
    patch_jcc8(c, jne_len, ret_false);
    patch_jcc8(c, jne_byte, ret_false);
    let end = c.len();
    em.register_helper(HELPER_STR_EQ, start, end);
}

/// Parses Win32 `GetCommandLineA()`:
/// `HELPER_ARGS_COUNT() -> rax = count`
/// `HELPER_ARGS_GET(rcx = target_idx) -> rax = ptr, rdx = len`
fn emit_args_helpers<E: EmitterIo>(em: &mut E) {
    align16(em.code_mut());
    let get_start = em.code_mut().len();
    em.code_mut()
        .extend_from_slice(&[0xF3, 0x0F, 0x1E, 0xFA, 0x55, 0x48, 0x89, 0xE5]);
    em.code_mut().extend_from_slice(&[0x48, 0x83, 0xEC, 0x30]);
    em.code_mut().extend_from_slice(&[0x48, 0x89, 0x4D, 0xF8]); // [RBP-8] = target_idx
    em.call_iat(IAT_GET_COMMAND_LINE_A);

    let c = em.code_mut();
    c.extend_from_slice(&[0x49, 0x89, 0xC0]); // MOV R8, RAX (cursor)
    c.extend_from_slice(&[0x45, 0x31, 0xC9]); // XOR R9D, R9D (cur_idx = 0)
    c.extend_from_slice(&[0x4C, 0x8B, 0x55, 0xF8]); // MOV R10, [RBP-8]

    let next_tok = c.len();
    let skip_ws = c.len();
    c.extend_from_slice(&[0x41, 0x0F, 0xB6, 0x00]); // MOVZX EAX, BYTE [R8]
    c.extend_from_slice(&[0x3C, b' ']);
    let je_sp = emit_jcc8(c, 0x74);
    c.extend_from_slice(&[0x3C, b'\t']);
    let jne_ws_done = emit_jcc8(c, 0x75);
    let do_inc_ws = c.len();
    patch_jcc8(c, je_sp, do_inc_ws);
    c.extend_from_slice(&[0x49, 0xFF, 0xC0]); // INC R8
    let jmp_ws = emit_jcc8(c, 0xEB);
    patch_jcc8(c, jmp_ws, skip_ws);

    let after_ws = c.len();
    patch_jcc8(c, jne_ws_done, after_ws);

    // Check NUL terminator: TEST AL, AL; JZ .done_all
    c.extend_from_slice(&[0x84, 0xC0, 0x0F, 0x84]);
    let jz_done_pos = c.len();
    c.extend_from_slice(&0i32.to_le_bytes());

    c.extend_from_slice(&[0x3C, b'"']);
    let jne_unquoted = emit_jcc8(c, 0x75);

    // Quoted token: INC R8; RCX = R8
    c.extend_from_slice(&[0x49, 0xFF, 0xC0, 0x4C, 0x89, 0xC1]);
    let q_loop = c.len();
    c.extend_from_slice(&[0x41, 0x0F, 0xB6, 0x00]);
    c.extend_from_slice(&[0x84, 0xC0]);
    let jz_q_end = emit_jcc8(c, 0x74);
    c.extend_from_slice(&[0x3C, b'"']);
    let je_q_end = emit_jcc8(c, 0x74);
    c.extend_from_slice(&[0x49, 0xFF, 0xC0]);
    let jmp_ql = emit_jcc8(c, 0xEB);
    patch_jcc8(c, jmp_ql, q_loop);

    let q_end = c.len();
    patch_jcc8(c, jz_q_end, q_end);
    patch_jcc8(c, je_q_end, q_end);
    c.extend_from_slice(&[0x4C, 0x89, 0xC2, 0x48, 0x29, 0xCA]); // RDX = R8 - RCX
    c.extend_from_slice(&[0x3C, b'"']);
    let jne_got_from_q = emit_jcc8(c, 0x75);
    c.extend_from_slice(&[0x49, 0xFF, 0xC0]); // INC R8 past closing '"'
    let jmp_got_from_q = emit_jcc8(c, 0xEB);

    let unquoted_pos = c.len();
    patch_jcc8(c, jne_unquoted, unquoted_pos);
    c.extend_from_slice(&[0x4C, 0x89, 0xC1]); // RCX = R8
    let uq_loop = c.len();
    c.extend_from_slice(&[0x41, 0x0F, 0xB6, 0x00]);
    c.extend_from_slice(&[0x84, 0xC0]);
    let jz_uq_end = emit_jcc8(c, 0x74);
    c.extend_from_slice(&[0x3C, b' ']);
    let je_uq_sp = emit_jcc8(c, 0x74);
    c.extend_from_slice(&[0x3C, b'\t']);
    let je_uq_tb = emit_jcc8(c, 0x74);
    c.extend_from_slice(&[0x49, 0xFF, 0xC0]);
    let jmp_uql = emit_jcc8(c, 0xEB);
    patch_jcc8(c, jmp_uql, uq_loop);

    let uq_end = c.len();
    patch_jcc8(c, jz_uq_end, uq_end);
    patch_jcc8(c, je_uq_sp, uq_end);
    patch_jcc8(c, je_uq_tb, uq_end);
    c.extend_from_slice(&[0x4C, 0x89, 0xC2, 0x48, 0x29, 0xCA]); // RDX = R8 - RCX

    let got_tok = c.len();
    patch_jcc8(c, jne_got_from_q, got_tok);
    patch_jcc8(c, jmp_got_from_q, got_tok);

    c.extend_from_slice(&[0x4D, 0x39, 0xD1]); // CMP R9, R10
    let je_found = emit_jcc8(c, 0x74);
    c.extend_from_slice(&[0x49, 0xFF, 0xC1, 0xE9]); // INC R9; JMP next_tok
    let jb_pos = c.len();
    c.extend_from_slice(&0i32.to_le_bytes());
    let rel_next = (next_tok as isize - (jb_pos as isize + 4)) as i32;
    c[jb_pos..jb_pos + 4].copy_from_slice(&rel_next.to_le_bytes());

    let found_pos = c.len();
    patch_jcc8(c, je_found, found_pos);
    c.extend_from_slice(&[0x48, 0x89, 0xC8, 0x48, 0x89, 0xEC, 0x5D, 0xC3]);

    let done_pos = c.len();
    let rel_done = (done_pos as isize - (jz_done_pos as isize + 4)) as i32;
    c[jz_done_pos..jz_done_pos + 4].copy_from_slice(&rel_done.to_le_bytes());
    c.extend_from_slice(&[0x4D, 0x85, 0xD2]); // TEST R10, R10
    let jns_not_cnt = emit_jcc8(c, 0x79);
    c.extend_from_slice(&[0x4C, 0x89, 0xC8, 0x48, 0x89, 0xEC, 0x5D, 0xC3]);
    let zero_ret = c.len();
    patch_jcc8(c, jns_not_cnt, zero_ret);
    c.extend_from_slice(&[0x31, 0xC0, 0x31, 0xD2, 0x48, 0x89, 0xEC, 0x5D, 0xC3]);
    let get_end = c.len();
    em.register_helper(HELPER_ARGS_GET, get_start, get_end);

    align16(em.code_mut());
    let cnt_start = em.code_mut().len();
    let c2 = em.code_mut();
    c2.extend_from_slice(&[0xF3, 0x0F, 0x1E, 0xFA]);
    c2.extend_from_slice(&[0x48, 0xC7, 0xC1, 0xFF, 0xFF, 0xFF, 0xFF]); // MOV RCX, -1
    c2.push(0xE9);
    let jmp_p = c2.len();
    c2.extend_from_slice(&0i32.to_le_bytes());
    let rel_g = (get_start as isize - (jmp_p as isize + 4)) as i32;
    c2[jmp_p..jmp_p + 4].copy_from_slice(&rel_g.to_le_bytes());
    let cnt_end = c2.len();
    em.register_helper(HELPER_ARGS_COUNT, cnt_start, cnt_end);
}

/// `HELPER_FS_READ(rcx = path_ptr, rdx = path_len) -> rax = err_tag (0=ok, 1=err), rdx = ByteBuf ptr`
fn emit_fs_read_helper<E: EmitterIo>(em: &mut E) {
    align16(em.code_mut());
    let start = em.code_mut().len();
    // Reserve 0x280 bytes frame: [RBP - 0x220 .. RBP - 0x20] = 512B null-terminated path buffer!
    em.code_mut()
        .extend_from_slice(&[0xF3, 0x0F, 0x1E, 0xFA, 0x55, 0x48, 0x89, 0xE5]);
    em.code_mut()
        .extend_from_slice(&[0x48, 0x81, 0xEC, 0x80, 0x02, 0x00, 0x00]);

    // Copy path (up to 500 bytes) into [RBP - 0x220] and append '\0'
    {
        let c = em.code_mut();
        c.extend_from_slice(&[0x4C, 0x8D, 0x85, 0xE0, 0xFD, 0xFF, 0xFF]); // LEA R8, [RBP - 0x220]
        c.extend_from_slice(&[0x31, 0xC0]); // XOR EAX, EAX
        let cp_loop = c.len();
        c.extend_from_slice(&[0x48, 0x39, 0xD0]); // CMP RAX, RDX
        let jae1 = emit_jcc8(c, 0x73);
        c.extend_from_slice(&[0x48, 0x3D, 0xF4, 0x01, 0x00, 0x00]); // CMP RAX, 500
        let jae2 = emit_jcc8(c, 0x73);
        c.extend_from_slice(&[0x44, 0x8A, 0x0C, 0x01, 0x45, 0x88, 0x0C, 0x00]); // MOV R9B, [RCX+RAX]; MOV [R8+RAX], R9B
        c.extend_from_slice(&[0x48, 0xFF, 0xC0]); // INC RAX
        let jmp_cp = emit_jcc8(c, 0xEB);
        patch_jcc8(c, jmp_cp, cp_loop);
        let cp_done = c.len();
        patch_jcc8(c, jae1, cp_done);
        patch_jcc8(c, jae2, cp_done);
        c.extend_from_slice(&[0x41, 0xC6, 0x04, 0x00, 0x00]); // MOV BYTE [R8 + RAX], 0

        // CreateFileA(R8, GENERIC_READ=0x80000000, FILE_SHARE_READ=1, NULL=0, OPEN_EXISTING=3, NORMAL=0x80, NULL=0)
        c.extend_from_slice(&[0x4C, 0x89, 0xC1]); // MOV RCX, R8
        c.extend_from_slice(&[0xBA, 0x00, 0x00, 0x00, 0x80]); // MOV EDX, 0x80000000
        c.extend_from_slice(&[0x41, 0xB8, 0x01, 0x00, 0x00, 0x00]); // MOV R8D, 1
        c.extend_from_slice(&[0x45, 0x31, 0xC9]); // XOR R9D, R9D
        c.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x20, 0x03, 0x00, 0x00, 0x00]); // [RSP+0x20] = 3
        c.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x28, 0x80, 0x00, 0x00, 0x00]); // [RSP+0x28] = 0x80
        c.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x30, 0x00, 0x00, 0x00, 0x00]); // [RSP+0x30] = 0
    }
    em.call_iat(IAT_CREATE_FILE_A);

    // Check RAX == -1 (INVALID_HANDLE_VALUE)
    let j_err_pos = {
        let c = em.code_mut();
        c.extend_from_slice(&[0x48, 0x83, 0xF8, 0xFF, 0x0F, 0x84]); // CMP RAX, -1; JE .err
        let p = c.len();
        c.extend_from_slice(&0i32.to_le_bytes());
        // Save hFile in [RBP - 8]
        c.extend_from_slice(&[0x48, 0x89, 0x45, 0xF8]);
        // GetFileSizeEx(hFile, &[RBP - 16])
        c.extend_from_slice(&[0x48, 0x89, 0xC1, 0x48, 0x8D, 0x55, 0xF0]);
        p
    };
    em.call_iat(IAT_GET_FILE_SIZE_EX);

    // VirtualAlloc(0, file_size + 64, 0x3000, 0x04)
    {
        let c = em.code_mut();
        c.extend_from_slice(&[0x31, 0xC9]); // XOR ECX, ECX
        c.extend_from_slice(&[0x48, 0x8B, 0x55, 0xF0]); // MOV RDX, [RBP-16]
        c.extend_from_slice(&[0x48, 0x83, 0xC2, 0x40]); // ADD RDX, 64
        c.extend_from_slice(&[0x41, 0xB8, 0x00, 0x30, 0x00, 0x00]);
        c.extend_from_slice(&[0x41, 0xB9, 0x04, 0x00, 0x00, 0x00]);
    }
    em.call_iat(IAT_VIRTUAL_ALLOC);

    // Save buf_ptr in [RBP - 24], set [buf_ptr + 8] = file_size + 32
    {
        let c = em.code_mut();
        c.extend_from_slice(&[0x48, 0x89, 0x45, 0xE8]); // [RBP-24] = buf_ptr
        c.extend_from_slice(&[0x48, 0x8B, 0x55, 0xF0, 0x48, 0x83, 0xC2, 0x20]);
        c.extend_from_slice(&[0x48, 0x89, 0x50, 0x08]); // [buf + 8] = cap
        // ReadFile(hFile=[RBP-8], lpBuffer=buf+16, nBytes=[RBP-16], lpBytesRead=&[RBP-32], NULL=0)
        c.extend_from_slice(&[0x48, 0x8B, 0x4D, 0xF8]); // RCX = hFile
        c.extend_from_slice(&[0x48, 0x8D, 0x50, 0x10]); // RDX = buf + 16
        c.extend_from_slice(&[0x44, 0x8B, 0x45, 0xF0]); // R8D = file_size
        c.extend_from_slice(&[0x4C, 0x8D, 0x4D, 0xE0]); // R9 = &[RBP-32]
        c.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x20, 0x00, 0x00, 0x00, 0x00]);
        c.extend_from_slice(&[0x48, 0xC7, 0x45, 0xE0, 0x00, 0x00, 0x00, 0x00]);
    }
    em.call_iat(IAT_READ_FILE);

    // Set [buf + 0] = bytes_read ([RBP-32]); CloseHandle(hFile)
    {
        let c = em.code_mut();
        c.extend_from_slice(&[0x48, 0x8B, 0x45, 0xE8]); // RAX = buf
        c.extend_from_slice(&[0x8B, 0x55, 0xE0]);       // EDX = bytes_read (u32 -> u64)
        c.extend_from_slice(&[0x48, 0x89, 0x10]);       // [buf + 0] = bytes_read
        c.extend_from_slice(&[0x48, 0x8B, 0x4D, 0xF8]); // RCX = hFile
    }
    em.call_iat(IAT_CLOSE_HANDLE);

    let c = em.code_mut();
    // Return OK: RAX = 0 (err_tag = 0), RDX = [RBP - 24] (ByteBuf ptr!)
    c.extend_from_slice(&[0x31, 0xC0, 0x48, 0x8B, 0x55, 0xE8]);
    c.extend_from_slice(&[0x48, 0x89, 0xEC, 0x5D, 0xC3]);

    // .err: RAX = 1 (err_tag = 1), RDX = 0
    let err_off = c.len();
    let rel_err = (err_off as isize - (j_err_pos as isize + 4)) as i32;
    c[j_err_pos..j_err_pos + 4].copy_from_slice(&rel_err.to_le_bytes());
    c.extend_from_slice(&[0xB8, 0x01, 0x00, 0x00, 0x00, 0x31, 0xD2]);
    c.extend_from_slice(&[0x48, 0x89, 0xEC, 0x5D, 0xC3]);
    let end = c.len();
    em.register_helper(HELPER_FS_READ, start, end);
}

/// `HELPER_FS_WRITE(rcx = path_ptr, rdx = path_len, r8 = bytebuf_ptr) -> rax = err_tag, rdx = written`
fn emit_fs_write_helper<E: EmitterIo>(em: &mut E) {
    align16(em.code_mut());
    let start = em.code_mut().len();
    em.code_mut()
        .extend_from_slice(&[0xF3, 0x0F, 0x1E, 0xFA, 0x55, 0x48, 0x89, 0xE5]);
    em.code_mut()
        .extend_from_slice(&[0x48, 0x81, 0xEC, 0x80, 0x02, 0x00, 0x00]);

    {
        let c = em.code_mut();
        // Save bytebuf_ptr in [RBP - 24]
        c.extend_from_slice(&[0x4C, 0x89, 0x45, 0xE8]);
        // Copy path into [RBP - 0x220] and NUL-terminate
        c.extend_from_slice(&[0x4C, 0x8D, 0x85, 0xE0, 0xFD, 0xFF, 0xFF]); // LEA R8, [RBP-0x220]
        c.extend_from_slice(&[0x31, 0xC0]);
        let cp_loop = c.len();
        c.extend_from_slice(&[0x48, 0x39, 0xD0]);
        let jae1 = emit_jcc8(c, 0x73);
        c.extend_from_slice(&[0x48, 0x3D, 0xF4, 0x01, 0x00, 0x00]);
        let jae2 = emit_jcc8(c, 0x73);
        c.extend_from_slice(&[0x44, 0x8A, 0x0C, 0x01, 0x45, 0x88, 0x0C, 0x00]);
        c.extend_from_slice(&[0x48, 0xFF, 0xC0]);
        let jmp_cp = emit_jcc8(c, 0xEB);
        patch_jcc8(c, jmp_cp, cp_loop);
        let cp_done = c.len();
        patch_jcc8(c, jae1, cp_done);
        patch_jcc8(c, jae2, cp_done);
        c.extend_from_slice(&[0x41, 0xC6, 0x04, 0x00, 0x00]);

        // CreateFileA(R8, GENERIC_WRITE=0x40000000, 0, NULL, CREATE_ALWAYS=2, NORMAL=0x80, NULL=0)
        c.extend_from_slice(&[0x4C, 0x89, 0xC1]);
        c.extend_from_slice(&[0xBA, 0x00, 0x00, 0x00, 0x40]); // GENERIC_WRITE
        c.extend_from_slice(&[0x45, 0x31, 0xC0]); // R8D = 0
        c.extend_from_slice(&[0x45, 0x31, 0xC9]); // R9D = 0
        c.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x20, 0x02, 0x00, 0x00, 0x00]); // CREATE_ALWAYS = 2
        c.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x28, 0x80, 0x00, 0x00, 0x00]);
        c.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x30, 0x00, 0x00, 0x00, 0x00]);
    }
    em.call_iat(IAT_CREATE_FILE_A);

    let j_err_pos = {
        let c = em.code_mut();
        c.extend_from_slice(&[0x48, 0x83, 0xF8, 0xFF, 0x0F, 0x84]);
        let p = c.len();
        c.extend_from_slice(&0i32.to_le_bytes());
        c.extend_from_slice(&[0x48, 0x89, 0x45, 0xF8]); // [RBP-8] = hFile
        // R10 = bytebuf_ptr ([RBP-24])
        c.extend_from_slice(&[0x4C, 0x8B, 0x55, 0xE8]);
        // WriteFile(hFile=RAX, lpBuf=R10+16, nBytes=[R10+0], lpWritten=&[RBP-32], NULL=0)
        c.extend_from_slice(&[0x48, 0x89, 0xC1]);             // RCX = hFile
        c.extend_from_slice(&[0x49, 0x8D, 0x52, 0x10]);       // RDX = R10 + 16
        c.extend_from_slice(&[0x45, 0x8B, 0x02]);             // R8D = [R10 + 0]
        c.extend_from_slice(&[0x4C, 0x8D, 0x4D, 0xE0]);       // R9 = &[RBP-32]
        c.extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x20, 0x00, 0x00, 0x00, 0x00]);
        c.extend_from_slice(&[0x48, 0xC7, 0x45, 0xE0, 0x00, 0x00, 0x00, 0x00]);
        p
    };
    em.call_iat(IAT_WRITE_FILE);

    {
        let c = em.code_mut();
        c.extend_from_slice(&[0x48, 0x8B, 0x4D, 0xF8]); // RCX = hFile
    }
    em.call_iat(IAT_CLOSE_HANDLE);

    let c = em.code_mut();
    c.extend_from_slice(&[0x31, 0xC0, 0x8B, 0x55, 0xE0]); // RAX = 0, RDX = written
    c.extend_from_slice(&[0x48, 0x89, 0xEC, 0x5D, 0xC3]);

    let err_off = c.len();
    let rel_err = (err_off as isize - (j_err_pos as isize + 4)) as i32;
    c[j_err_pos..j_err_pos + 4].copy_from_slice(&rel_err.to_le_bytes());
    c.extend_from_slice(&[0xB8, 0x01, 0x00, 0x00, 0x00, 0x31, 0xD2]);
    c.extend_from_slice(&[0x48, 0x89, 0xEC, 0x5D, 0xC3]);
    let end = c.len();
    em.register_helper(HELPER_FS_WRITE, start, end);
}
