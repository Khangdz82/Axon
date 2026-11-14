//! Axon v5.0 Direct Native x86_64 + AVX2 SIMD Machine Code Compiler (`x86_64.rs`)
//! Compiles AST + CheckedProgram directly into native x86_64 instructions with:
//! - Hardware Intel CET (`ENDBR64`)
//! - `@shield` Live `.text` FNV-1a Machine-Code Integrity Verification + `@whitebox` GF(2^8) Tables
//! - `@secret` Compile-Time Scrambling + Stack Reconstruction + Immediate Epilogue Zeroization
//! - Static Perceus Deterministic `Drop` (`VirtualFree`) + Zero-Syscall FBIP In-Place Memory Reuse
//! - Hardware Checked Arithmetic (`JO` trap), Div-by-Zero Guard, and Bounds Guard
//! - Native SoA `Table[T]` Columnar Allocation + AVX2/FMA3 (`VFMADD231PS`) 256-bit Vectorization

use crate::ast::*;
use crate::pe::*;
use crate::perceus::{analyze_program_perceus, PerceusFnPlan};
use crate::runtime_io::{
    self, EmitterIo, HELPER_ARGS_COUNT, HELPER_ARGS_GET, HELPER_FS_READ, HELPER_FS_WRITE,
    HELPER_STR_EQ,
};
use crate::sema::*;
use crate::soa_simd::{analyze_foreach_simd, SimdKernelOp};
use crate::whitebox::generate_whitebox_table;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
enum FixupTarget {
    Iat(usize),
    RdataOff(usize), // Offset relative to `rdata_layout.data_start_rva`
    ShieldExpectedHash,
    InternalFn(usize), // Function index (0..funcs.len() or runtime helper id)
}

#[derive(Debug, Clone, Copy)]
struct Fixup {
    disp_pos: usize,
    next_rip: usize,
    target: FixupTarget,
}

const HELPER_PRINT_BYTES: usize = 100_000;
const HELPER_PRINT_I64: usize = 100_001;
const HELPER_PRINT_DEC: usize = 100_002;
const HELPER_CLOCK_NS: usize = 100_003;
const HELPER_SEC_TRAP: usize = 100_004;
const HELPER_SHIELD_TRAP: usize = 100_005;

pub struct CompiledModule {
    pub exe_bytes: Vec<u8>,
    pub text_size: usize,
    pub fn_count: usize,
    pub simd_loops_emitted: usize,
    pub dead_cols_eliminated: usize,
    pub perceus_reuses: usize,
    pub perceus_drops: usize,
    pub whitebox_tables: usize,
    pub secrets_encrypted: usize,
    pub shield_enabled: bool,
}

#[derive(Clone)]
struct FnMeta {
    idx: usize,
    use_fast_abi: bool,
    param_tys: Vec<Ty>,
    param_offsets: Vec<usize>, // Slot offsets in arg buffer
    total_arg_slots: usize,
    ret_ty: Ty,
    ret_slots: usize,
}

pub fn compile_to_pe(prog: &Program, checked: &CheckedProgram, force_shield: bool) -> CompiledModule {
    let shield_enabled = force_shield
        || prog.module_attrs.iter().any(|a| a == "shield")
        || prog
            .funcs
            .iter()
            .any(|f| f.attrs.iter().any(|(a, _)| a == "shield" || a == "whitebox"));

    let perceus_plans = analyze_program_perceus(prog, checked);

    let mut cg = ModuleCodegen {
        prog,
        checked,
        perceus_plans,
        shield_enabled,
        code: Vec::with_capacity(4096),
        rdata_extra: Vec::with_capacity(512),
        fixups: Vec::new(),
        fn_offsets: HashMap::new(),
        fn_ranges: Vec::new(),
        fn_metas: HashMap::new(),
        str_pool: HashMap::new(),
        whitebox_pool: HashMap::new(),
        simd_loops_emitted: 0,
        dead_cols_eliminated: 0,
        perceus_reuses: 0,
        perceus_drops: 0,
        secrets_encrypted: 0,
    };

    cg.prepare_fn_metas();
    cg.emit_all()
}

struct ModuleCodegen<'a> {
    prog: &'a Program,
    checked: &'a CheckedProgram,
    perceus_plans: HashMap<String, PerceusFnPlan>,
    shield_enabled: bool,
    code: Vec<u8>,
    rdata_extra: Vec<u8>,
    fixups: Vec<Fixup>,
    fn_offsets: HashMap<usize, usize>,
    fn_ranges: Vec<(u32, u32)>,
    fn_metas: HashMap<String, FnMeta>,
    str_pool: HashMap<String, (usize, usize)>,
    whitebox_pool: HashMap<u64, usize>,
    simd_loops_emitted: usize,
    dead_cols_eliminated: usize,
    perceus_reuses: usize,
    perceus_drops: usize,
    secrets_encrypted: usize,
}

impl<'a> EmitterIo for ModuleCodegen<'a> {
    fn code_mut(&mut self) -> &mut Vec<u8> {
        &mut self.code
    }
    fn register_helper(&mut self, id: usize, start: usize, end: usize) {
        self.fn_offsets.insert(id, start);
        self.fn_ranges.push((start as u32, end as u32));
    }
    fn call_iat(&mut self, slot: usize) {
        self.emit_call_iat(slot);
    }
}

impl<'a> ModuleCodegen<'a> {
    fn sorted_struct_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.checked.structs.keys().cloned().collect();
        names.sort();
        names
    }

    fn struct_type_id(&self, sname: &str) -> i64 {
        self.sorted_struct_names()
            .iter()
            .position(|n| n == sname)
            .map(|i| (i + 1) as i64)
            .unwrap_or(0)
    }

    fn type_slots(&self, ty: &Ty) -> usize {
        match ty {
            Ty::Unit => 1,
            Ty::Int | Ty::Float | Ty::Dec | Ty::Bool | Ty::Job(_) | Ty::Any => 1,
            Ty::Interface(_) => {
                let max_s = self
                    .checked
                    .structs
                    .values()
                    .map(|l| {
                        l.fields
                            .iter()
                            .map(|(_, fty)| match fty {
                                Ty::Interface(_) => 2,
                                other => self.type_slots(other),
                            })
                            .sum::<usize>()
                            .max(1)
                    })
                    .max()
                    .unwrap_or(1);
                1 + max_s
            }
            Ty::Str | Ty::Table(_) => 2,
            Ty::Result(ok, _) => 1 + self.type_slots(ok).max(1),
            Ty::Tuple(elems) => elems.iter().map(|e| self.type_slots(e)).sum::<usize>().max(1),
            Ty::Struct(sname) => {
                if let Some(layout) = self.checked.structs.get(sname) {
                    layout
                        .fields
                        .iter()
                        .map(|(_, fty)| self.type_slots(fty))
                        .sum::<usize>()
                        .max(1)
                } else {
                    1
                }
            }
        }
    }

    fn struct_field_slot_and_ty(&self, sname: &str, field: &str) -> (usize, Ty) {
        if let Some(layout) = self.checked.structs.get(sname) {
            let mut offset = 0usize;
            for (fname, fty) in &layout.fields {
                if fname == field {
                    return (offset, fty.clone());
                }
                offset += self.type_slots(fty);
            }
        }
        (0, Ty::Int)
    }

    fn prepare_fn_metas(&mut self) {
        for (idx, f) in self.prog.funcs.iter().enumerate() {
            let sig = self.checked.fn_sigs.get(&f.name);
            let mut param_tys = Vec::new();
            let mut param_offsets = Vec::new();
            let mut total_arg_slots = 0usize;
            let mut any_mut_param = false;

            for (p_i, p) in f.params.iter().enumerate() {
                if matches!(p.mode, ParamMode::Mut) {
                    any_mut_param = true;
                }
                let pty = sig
                    .and_then(|s| s.params.get(p_i).map(|x| x.2.clone()))
                    .unwrap_or_else(|| resolve_typespec(&p.ty, &self.checked.structs, &self.checked.interfaces));
                let slots = self.type_slots(&pty);
                param_offsets.push(total_arg_slots);
                total_arg_slots += slots;
                param_tys.push(pty);
            }

            let ret_ty = sig
                .map(|s| s.ret.clone())
                .unwrap_or_else(|| resolve_typespec(&f.ret_ty, &self.checked.structs, &self.checked.interfaces));
            let ret_slots = self.type_slots(&ret_ty);

            let use_fast_abi = !any_mut_param
                && param_tys.len() <= 4
                && param_tys.iter().all(|t| self.type_slots(t) == 1)
                && ret_slots <= 2;

            self.fn_metas.insert(
                f.name.clone(),
                FnMeta {
                    idx,
                    use_fast_abi,
                    param_tys,
                    param_offsets,
                    total_arg_slots,
                    ret_ty,
                    ret_slots,
                },
            );
        }
    }

    fn intern_str(&mut self, s: &str) -> (usize, usize) {
        if let Some(&loc) = self.str_pool.get(s) {
            return loc;
        }
        let off = self.rdata_extra.len();
        self.rdata_extra.extend_from_slice(s.as_bytes());
        self.rdata_extra.push(0);
        let loc = (off, s.len());
        self.str_pool.insert(s.to_string(), loc);
        loc
    }

    fn add_encrypted_secret(&mut self, plain: &str) -> (usize, usize, u64) {
        let site_id = self.secrets_encrypted as u64 + 1;
        self.secrets_encrypted += 1;
        let key: u64 = 0x9E37_79B9_7F4A_7C15u64
            ^ site_id.wrapping_mul(0xBF58_476D_1CE4_E5B9u64)
            ^ ((plain.len() as u64) << 32);

        let off = self.rdata_extra.len();
        for (i, &b) in plain.as_bytes().iter().enumerate() {
            let ks = (key.rotate_left(((i as u32) * 7) & 63) as u8)
                .wrapping_add((i as u8).wrapping_mul(37));
            self.rdata_extra.push(b ^ ks);
        }
        (off, plain.len(), key)
    }

    fn intern_whitebox_table(&mut self, seed_key: u64) -> usize {
        if let Some(&off) = self.whitebox_pool.get(&seed_key) {
            return off;
        }
        while self.rdata_extra.len() % 16 != 0 {
            self.rdata_extra.push(0);
        }
        let off = self.rdata_extra.len();
        let tbl = generate_whitebox_table(seed_key);
        self.rdata_extra.extend_from_slice(&tbl);
        self.whitebox_pool.insert(seed_key, off);
        off
    }

    fn emit_all(&mut self) -> CompiledModule {
        // 1. Emit `_start` at offset 0 (RVA 0x1000)
        let start_begin = self.code.len();
        let shield_len_imm_pos = self.emit_entry_start();
        let start_end = self.code.len();
        self.fn_ranges.push((start_begin as u32, start_end as u32));

        // 2. Emit built-in zero-CRT runtime helpers + OS I/O & CLI helpers
        self.emit_helper_print_bytes();
        self.emit_helper_print_i64();
        self.emit_helper_print_dec();
        self.emit_helper_clock_ns();
        self.emit_helper_traps();
        runtime_io::emit_extended_helpers(self);

        // 3. Emit all user & stdlib functions
        for (idx, f) in self.prog.funcs.iter().enumerate() {
            while self.code.len() % 16 != 0 {
                self.code.push(0xCC); // INT3 alignment padding
            }
            let fn_start = self.code.len();
            self.fn_offsets.insert(idx, fn_start);
            self.compile_function(idx, f);
            let fn_end = self.code.len();
            self.fn_ranges.push((fn_start as u32, fn_end as u32));
        }

        let shield_verified_len = self.code.len();
        if let Some(imm_pos) = shield_len_imm_pos {
            let len_bytes = (shield_verified_len as u32).to_le_bytes();
            self.code[imm_pos..imm_pos + 4].copy_from_slice(&len_bytes);
        }

        // 4. Compute RVA of `.rdata` dynamically after `.text`
        let text_aligned_vsize = ((self.code.len() as u32 + 0xFFF) & !0xFFF).max(0x1000);
        let rdata_rva = TEXT_RVA + text_aligned_vsize;
        let rdata_layout = prepare_rdata_prefix(rdata_rva);

        // 5. Resolve all RIP-relative & relative-call fixups in `.text`
        for fx in &self.fixups {
            let next_rip_rva = TEXT_RVA as i64 + fx.next_rip as i64;
            let target_rva: i64 = match fx.target {
                FixupTarget::Iat(slot) => rdata_layout.iat_entry_rvas[slot] as i64,
                FixupTarget::RdataOff(off) => rdata_layout.data_start_rva as i64 + off as i64,
                FixupTarget::ShieldExpectedHash => rdata_layout.shield_hash_rva as i64,
                FixupTarget::InternalFn(fn_id) => {
                    let off = *self.fn_offsets.get(&fn_id).unwrap_or(&0);
                    TEXT_RVA as i64 + off as i64
                }
            };
            let disp = (target_rva - next_rip_rva) as i32;
            self.code[fx.disp_pos..fx.disp_pos + 4].copy_from_slice(&disp.to_le_bytes());
        }

        let mut rdata_full = rdata_layout.prefix_bytes.clone();
        rdata_full.extend_from_slice(&self.rdata_extra);

        let text_size = self.code.len();
        let exe_bytes = build_pe_executable(
            &self.code,
            shield_verified_len,
            rdata_full,
            &rdata_layout,
            &self.fn_ranges,
        );

        CompiledModule {
            exe_bytes,
            text_size,
            fn_count: self.prog.funcs.len(),
            simd_loops_emitted: self.simd_loops_emitted,
            dead_cols_eliminated: self.dead_cols_eliminated,
            perceus_reuses: self.perceus_reuses,
            perceus_drops: self.perceus_drops,
            whitebox_tables: self.whitebox_pool.len(),
            secrets_encrypted: self.secrets_encrypted,
            shield_enabled: self.shield_enabled,
        }
    }

    fn emit_call_fn(&mut self, fn_id: usize) {
        self.code.push(0xE8);
        let disp_pos = self.code.len();
        self.code.extend_from_slice(&0i32.to_le_bytes());
        let next_rip = self.code.len();
        self.fixups.push(Fixup {
            disp_pos,
            next_rip,
            target: FixupTarget::InternalFn(fn_id),
        });
    }

    fn emit_call_iat(&mut self, slot: usize) {
        self.code.extend_from_slice(&[0xFF, 0x15]); // CALL QWORD PTR [RIP + disp32]
        let disp_pos = self.code.len();
        self.code.extend_from_slice(&0i32.to_le_bytes());
        let next_rip = self.code.len();
        self.fixups.push(Fixup {
            disp_pos,
            next_rip,
            target: FixupTarget::Iat(slot),
        });
    }

    fn emit_lea_rax_rdata(&mut self, rdata_off: usize) {
        self.code.extend_from_slice(&[0x48, 0x8D, 0x05]); // LEA RAX, [RIP + disp32]
        let disp_pos = self.code.len();
        self.code.extend_from_slice(&0i32.to_le_bytes());
        let next_rip = self.code.len();
        self.fixups.push(Fixup {
            disp_pos,
            next_rip,
            target: FixupTarget::RdataOff(rdata_off),
        });
    }

    fn emit_jmp_fn(&mut self, op2: u8, fn_id: usize) {
        self.code.extend_from_slice(&[0x0F, op2]); // Jcc rel32
        let disp_pos = self.code.len();
        self.code.extend_from_slice(&0i32.to_le_bytes());
        let next_rip = self.code.len();
        self.fixups.push(Fixup {
            disp_pos,
            next_rip,
            target: FixupTarget::InternalFn(fn_id),
        });
    }

    fn emit_entry_start(&mut self) -> Option<usize> {
        // ENDBR64 + PUSH RBP + MOV RBP, RSP + SUB RSP, 0x40
        self.code
            .extend_from_slice(&[0xF3, 0x0F, 0x1E, 0xFA, 0x55, 0x48, 0x89, 0xE5]);
        self.code
            .extend_from_slice(&[0x48, 0x83, 0xEC, 0x40]);

        let mut shield_len_pos = None;
        if self.shield_enabled {
            // Live `.text` FNV-1a 64-bit Machine-Code Integrity Verification
            // LEA RCX, [RIP + disp_to_0]
            self.code.extend_from_slice(&[0x48, 0x8D, 0x0D]);
            let disp_to_zero = -(self.code.len() as i32 + 4);
            self.code.extend_from_slice(&disp_to_zero.to_le_bytes());

            // MOV EDX, imm32 (patched with `shield_verified_len` once `.text` finishes!)
            self.code.push(0xBA);
            shield_len_pos = Some(self.code.len());
            self.code.extend_from_slice(&0u32.to_le_bytes());

            // MOV RAX, 0xcbf29ce484222325
            self.code.extend_from_slice(&[0x48, 0xB8]);
            self.code
                .extend_from_slice(&0xcbf29ce484222325u64.to_le_bytes());
            // MOV R8, 0x100000001b3
            self.code.extend_from_slice(&[0x49, 0xB8]);
            self.code
                .extend_from_slice(&0x0000_0100_0000_01b3u64.to_le_bytes());

            let loop_top = self.code.len();
            // MOVZX R9D, BYTE PTR [RCX] (44 0F B6 09)
            self.code.extend_from_slice(&[0x44, 0x0F, 0xB6, 0x09]);
            // XOR RAX, R9 (4C 31 C8)
            self.code.extend_from_slice(&[0x4C, 0x31, 0xC8]);
            // IMUL RAX, R8 (49 0F AF C0)
            self.code.extend_from_slice(&[0x49, 0x0F, 0xAF, 0xC0]);
            // INC RCX (48 FF C1)
            self.code.extend_from_slice(&[0x48, 0xFF, 0xC1]);
            // DEC EDX (FF CA)
            self.code.extend_from_slice(&[0xFF, 0xCA]);
            // JNZ loop_top
            self.code.push(0x75);
            let rel8 = (loop_top as isize - (self.code.len() as isize + 1)) as i8;
            self.code.push(rel8 as u8);

            // MOV RCX, QWORD PTR [RIP + shield_expected_hash]
            self.code.extend_from_slice(&[0x48, 0x8B, 0x0D]);
            let disp_pos = self.code.len();
            self.code.extend_from_slice(&0i32.to_le_bytes());
            let next_rip = self.code.len();
            self.fixups.push(Fixup {
                disp_pos,
                next_rip,
                target: FixupTarget::ShieldExpectedHash,
            });
            // CMP RAX, RCX -> JNE __axon_shield_trap
            self.code.extend_from_slice(&[0x48, 0x39, 0xC8]);
            self.emit_jmp_fn(0x85, HELPER_SHIELD_TRAP);
        }

        // Call `main` if present
        if let Some(main_meta) = self.fn_metas.get("main").cloned() {
            if !main_meta.use_fast_abi {
                // Provide dummy arg/ret buffer on stack `[RBP - 0x20]`
                self.code.extend_from_slice(&[0x48, 0x8D, 0x4D, 0xE0]); // LEA RCX, [RBP-32]
                self.code.extend_from_slice(&[0x48, 0x8D, 0x55, 0xE0]); // LEA RDX, [RBP-32]
            }
            self.emit_call_fn(main_meta.idx);
        } else {
            self.code.extend_from_slice(&[0x31, 0xC0]); // XOR EAX, EAX
        }

        // MOV ECX, EAX -> CALL ExitProcess
        self.code.extend_from_slice(&[0x89, 0xC1]);
        self.emit_call_iat(IAT_EXIT_PROCESS);
        self.code.push(0xCC);
        shield_len_pos
    }

    fn emit_helper_print_bytes(&mut self) {
        while self.code.len() % 16 != 0 {
            self.code.push(0xCC);
        }
        let start = self.code.len();
        self.fn_offsets.insert(HELPER_PRINT_BYTES, start);
        // RCX = ptr, RDX = len
        // ENDBR64; PUSH RBP; MOV RBP, RSP; SUB RSP, 0x50
        self.code
            .extend_from_slice(&[0xF3, 0x0F, 0x1E, 0xFA, 0x55, 0x48, 0x89, 0xE5]);
        self.code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x50]);
        // Save ptr in [RBP-8], len in [RBP-16]
        self.code.extend_from_slice(&[0x48, 0x89, 0x4D, 0xF8]);
        self.code.extend_from_slice(&[0x48, 0x89, 0x55, 0xF0]);
        // MOV ECX, -11 (STD_OUTPUT_HANDLE = 0xFFFFFFF5)
        self.code
            .extend_from_slice(&[0xB9, 0xF5, 0xFF, 0xFF, 0xFF]);
        self.emit_call_iat(IAT_GET_STD_HANDLE);
        // RCX = RAX (hOut), RDX = [RBP-8] (ptr), R8D = [RBP-16] (len), R9 = LEA [RBP-24] (&written), [RSP+0x20] = 0
        self.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // MOV RCX, RAX
        self.code.extend_from_slice(&[0x48, 0x8B, 0x55, 0xF8]); // MOV RDX, [RBP-8]
        self.code.extend_from_slice(&[0x44, 0x8B, 0x45, 0xF0]); // MOV R8D, [RBP-16]
        self.code.extend_from_slice(&[0x4C, 0x8D, 0x4D, 0xE8]); // LEA R9, [RBP-24]
        self.code
            .extend_from_slice(&[0x48, 0xC7, 0x44, 0x24, 0x20, 0x00, 0x00, 0x00, 0x00]); // QWORD [RSP+0x20] = 0
        self.emit_call_iat(IAT_WRITE_FILE);
        self.code
            .extend_from_slice(&[0x48, 0x89, 0xEC, 0x5D, 0xC3]); // MOV RSP, RBP; POP RBP; RET
        let end = self.code.len();
        self.fn_ranges.push((start as u32, end as u32));
    }

    fn emit_helper_print_i64(&mut self) {
        while self.code.len() % 16 != 0 {
            self.code.push(0xCC);
        }
        let start = self.code.len();
        self.fn_offsets.insert(HELPER_PRINT_I64, start);
        // RCX = signed i64
        self.code
            .extend_from_slice(&[0xF3, 0x0F, 0x1E, 0xFA, 0x55, 0x48, 0x89, 0xE5]);
        self.code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x50]);

        // Buffer ends at LEA R8, [RBP - 8]
        self.code.extend_from_slice(&[0x4C, 0x8D, 0x45, 0xF8]); // LEA R8, [RBP-8]
        self.code.extend_from_slice(&[0x4D, 0x89, 0xC1]); // MOV R9, R8 (end_ptr)
        self.code.extend_from_slice(&[0x45, 0x31, 0xD2]); // XOR R10D, R10D (is_neg = 0)
        self.code.extend_from_slice(&[0x48, 0x89, 0xC8]); // MOV RAX, RCX
        // TEST RAX, RAX; JGE .non_neg
        self.code.extend_from_slice(&[0x48, 0x85, 0xC0, 0x7D, 0x06]);
        // NEG RAX; MOV R10D, 1
        self.code
            .extend_from_slice(&[0x48, 0xF7, 0xD8, 0x41, 0xB2, 0x01]);

        // .non_neg: MOV R11, 10
        self.code
            .extend_from_slice(&[0x49, 0xC7, 0xC3, 0x0A, 0x00, 0x00, 0x00]);
        let digit_loop = self.code.len();
        // XOR EDX, EDX; DIV R11 (unsigned 64-bit div handles i64::MIN negated too!)
        self.code
            .extend_from_slice(&[0x31, 0xD2, 0x49, 0xF7, 0xF3]);
        // ADD DL, '0'; DEC R8; MOV [R8], DL
        self.code
            .extend_from_slice(&[0x80, 0xC2, 0x30, 0x49, 0xFF, 0xC8, 0x41, 0x88, 0x10]);
        // TEST RAX, RAX; JNZ digit_loop
        self.code.extend_from_slice(&[0x48, 0x85, 0xC0, 0x75]);
        let rel = (digit_loop as isize - (self.code.len() as isize + 1)) as i8;
        self.code.push(rel as u8);

        // TEST R10D, R10D; JZ .done_sign
        self.code.extend_from_slice(&[0x45, 0x85, 0xD2, 0x74, 0x07]);
        // DEC R8; MOV BYTE PTR [R8], '-'
        self.code
            .extend_from_slice(&[0x49, 0xFF, 0xC8, 0x41, 0xC6, 0x00, b'-']);

        // .done_sign: MOV RCX, R8; MOV RDX, R9; SUB RDX, R8 -> CALL __axon_print_bytes
        self.code
            .extend_from_slice(&[0x4C, 0x89, 0xC1, 0x4C, 0x89, 0xCA, 0x4C, 0x29, 0xC2]);
        self.emit_call_fn(HELPER_PRINT_BYTES);
        self.code
            .extend_from_slice(&[0x48, 0x89, 0xEC, 0x5D, 0xC3]);
        let end = self.code.len();
        self.fn_ranges.push((start as u32, end as u32));
    }

    fn emit_helper_print_dec(&mut self) {
        while self.code.len() % 16 != 0 {
            self.code.push(0xCC);
        }
        let start = self.code.len();
        self.fn_offsets.insert(HELPER_PRINT_DEC, start);
        // RCX = scaled fixed-point (scale = 10_000). Prints `int_part.XX` (or `.XXXX` if sub-cent non-zero)
        self.code
            .extend_from_slice(&[0xF3, 0x0F, 0x1E, 0xFA, 0x55, 0x48, 0x89, 0xE5]);
        self.code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x40]);

        // Save RCX in [RBP-8]
        self.code.extend_from_slice(&[0x48, 0x89, 0x4D, 0xF8]);
        // If negative, print '-' and negate
        self.code.extend_from_slice(&[0x48, 0x85, 0xC9, 0x7D]);
        let jge_pos = self.code.len();
        self.code.push(0);
        self.code
            .extend_from_slice(&[0xC6, 0x45, 0xEF, b'-', 0x48, 0x8D, 0x4D, 0xEF]);
        self.code
            .extend_from_slice(&[0xBA, 0x01, 0x00, 0x00, 0x00]);
        self.emit_call_fn(HELPER_PRINT_BYTES);
        self.code
            .extend_from_slice(&[0x48, 0x8B, 0x4D, 0xF8, 0x48, 0xF7, 0xD9, 0x48, 0x89, 0x4D, 0xF8]);
        let non_neg_pos = self.code.len();
        self.code[jge_pos] = (non_neg_pos - (jge_pos + 1)) as u8;

        // RAX = [RBP-8]; XOR EDX, EDX; MOV RCX, 10000; DIV RCX -> RAX = int_part, RDX = frac_0_9999
        self.code.extend_from_slice(&[0x48, 0x8B, 0x45, 0xF8, 0x31, 0xD2]);
        self.code
            .extend_from_slice(&[0x48, 0xC7, 0xC1, 0x10, 0x27, 0x00, 0x00]); // 10000
        self.code.extend_from_slice(&[0x48, 0xF7, 0xF1]);
        // Save frac in [RBP-16], print int_part (RCX = RAX)
        self.code.extend_from_slice(&[0x48, 0x89, 0x55, 0xF0, 0x48, 0x89, 0xC1]);
        self.emit_call_fn(HELPER_PRINT_I64);

        // Format `.XX` (2 decimal places = frac / 100) into [RBP-24 .. RBP-21]
        self.code.extend_from_slice(&[0xC6, 0x45, 0xE8, b'.']);
        self.code.extend_from_slice(&[0x48, 0x8B, 0x45, 0xF0, 0x31, 0xD2]);
        self.code
            .extend_from_slice(&[0x48, 0xC7, 0xC1, 0x64, 0x00, 0x00, 0x00]); // 100
        self.code.extend_from_slice(&[0x48, 0xF7, 0xF1]); // RAX = cents (0..99)
        self.code.extend_from_slice(&[0x31, 0xD2]);
        self.code
            .extend_from_slice(&[0x48, 0xC7, 0xC1, 0x0A, 0x00, 0x00, 0x00]); // 10
        self.code.extend_from_slice(&[0x48, 0xF7, 0xF1]); // AL = tens, DL = ones
        self.code
            .extend_from_slice(&[0x04, 0x30, 0x80, 0xC2, 0x30]);
        self.code
            .extend_from_slice(&[0x88, 0x45, 0xE9, 0x88, 0x55, 0xEA]);
        // LEA RCX, [RBP-24]; MOV EDX, 3 -> print_bytes
        self.code.extend_from_slice(&[0x48, 0x8D, 0x4D, 0xE8]);
        self.code
            .extend_from_slice(&[0xBA, 0x03, 0x00, 0x00, 0x00]);
        self.emit_call_fn(HELPER_PRINT_BYTES);

        self.code
            .extend_from_slice(&[0x48, 0x89, 0xEC, 0x5D, 0xC3]);
        let end = self.code.len();
        self.fn_ranges.push((start as u32, end as u32));
    }

    fn emit_helper_clock_ns(&mut self) {
        while self.code.len() % 16 != 0 {
            self.code.push(0xCC);
        }
        let start = self.code.len();
        self.fn_offsets.insert(HELPER_CLOCK_NS, start);
        self.code
            .extend_from_slice(&[0xF3, 0x0F, 0x1E, 0xFA, 0x55, 0x48, 0x89, 0xE5]);
        self.code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x40]);
        // QueryPerformanceCounter(&[RBP-8])
        self.code.extend_from_slice(&[0x48, 0x8D, 0x4D, 0xF8]);
        self.emit_call_iat(IAT_QPC);
        // QueryPerformanceFrequency(&[RBP-16])
        self.code.extend_from_slice(&[0x48, 0x8D, 0x4D, 0xF0]);
        self.emit_call_iat(IAT_QPF);
        // 128-bit `(counter * 1_000_000_000) / freq` -> RAX
        self.code.extend_from_slice(&[0x48, 0x8B, 0x45, 0xF8]); // MOV RAX, [RBP-8]
        self.code
            .extend_from_slice(&[0x48, 0xC7, 0xC1, 0x00, 0xCA, 0x9A, 0x3B]); // MOV RCX, 1_000_000_000
        self.code.extend_from_slice(&[0x48, 0xF7, 0xE1]); // MUL RCX -> RDX:RAX
        self.code.extend_from_slice(&[0x48, 0x8B, 0x4D, 0xF0]); // MOV RCX, [RBP-16]
        self.code.extend_from_slice(&[0x48, 0xF7, 0xF1]); // DIV RCX -> RAX = ns
        self.code
            .extend_from_slice(&[0x48, 0x89, 0xEC, 0x5D, 0xC3]);
        let end = self.code.len();
        self.fn_ranges.push((start as u32, end as u32));
    }

    fn emit_helper_traps(&mut self) {
        let (sec_off, sec_len) =
            self.intern_str("[AXON SECURITY] Memory/Arithmetic safety violation blocked\n");
        let (shd_off, shd_len) =
            self.intern_str("[AXON SHIELD] Code integrity verification mismatch blocked\n");

        // Security Trap (exit 101)
        while self.code.len() % 16 != 0 {
            self.code.push(0xCC);
        }
        let s1 = self.code.len();
        self.fn_offsets.insert(HELPER_SEC_TRAP, s1);
        self.code
            .extend_from_slice(&[0xF3, 0x0F, 0x1E, 0xFA, 0x48, 0x83, 0xEC, 0x28]);
        self.emit_lea_rax_rdata(sec_off);
        self.code.extend_from_slice(&[0x48, 0x89, 0xC1]);
        self.code.push(0xBA);
        self.code.extend_from_slice(&(sec_len as u32).to_le_bytes());
        self.emit_call_fn(HELPER_PRINT_BYTES);
        self.code
            .extend_from_slice(&[0xB9, 101, 0x00, 0x00, 0x00]);
        self.emit_call_iat(IAT_EXIT_PROCESS);
        self.code.push(0xCC);
        self.fn_ranges.push((s1 as u32, self.code.len() as u32));

        // Shield Trap (exit 255)
        while self.code.len() % 16 != 0 {
            self.code.push(0xCC);
        }
        let s2 = self.code.len();
        self.fn_offsets.insert(HELPER_SHIELD_TRAP, s2);
        self.code
            .extend_from_slice(&[0xF3, 0x0F, 0x1E, 0xFA, 0x48, 0x83, 0xEC, 0x28]);
        self.emit_lea_rax_rdata(shd_off);
        self.code.extend_from_slice(&[0x48, 0x89, 0xC1]);
        self.code.push(0xBA);
        self.code.extend_from_slice(&(shd_len as u32).to_le_bytes());
        self.emit_call_fn(HELPER_PRINT_BYTES);
        self.code
            .extend_from_slice(&[0xB9, 255, 0x00, 0x00, 0x00]);
        self.emit_call_iat(IAT_EXIT_PROCESS);
        self.code.push(0xCC);
        self.fn_ranges.push((s2 as u32, self.code.len() as u32));
    }

    fn compile_function(&mut self, fn_idx: usize, f: &FnDecl) {
        let meta = self.fn_metas.get(&f.name).unwrap().clone();
        let perceus_plan = self.perceus_plans.get(&f.name).cloned();
        let mut fcg = FnCodegen {
            mc: self,
            fn_idx,
            meta,
            perceus_plan,
            vars: HashMap::new(),
            next_slot: 0,
            arg_buf_ptr_slot: None,
            ret_buf_ptr_slot: None,
            secret_wipe_ranges: Vec::new(),
            loop_stack: Vec::new(),
            elide_bounds_vars: std::collections::HashSet::new(),
        };
        fcg.emit(f);
    }
}

#[derive(Clone)]
struct LocalVar {
    slot: usize, // 1-based index from RBP (`[RBP - (slot + i) * 8]`)
    ty: Ty,
}

struct LoopPatch {
    continue_target: usize,
    break_jumps: Vec<usize>,
}

struct FnCodegen<'a, 'b> {
    mc: &'a mut ModuleCodegen<'b>,
    #[allow(dead_code)]
    fn_idx: usize,
    meta: FnMeta,
    perceus_plan: Option<PerceusFnPlan>,
    vars: HashMap<String, LocalVar>,
    next_slot: usize,
    arg_buf_ptr_slot: Option<usize>,
    ret_buf_ptr_slot: Option<usize>,
    secret_wipe_ranges: Vec<(usize, usize)>,
    loop_stack: Vec<LoopPatch>,
    elide_bounds_vars: std::collections::HashSet<String>,
}

impl<'a, 'b> FnCodegen<'a, 'b> {
    fn alloc_slots(&mut self, count: usize) -> usize {
        let base = self.next_slot + 1;
        self.next_slot += count.max(1);
        base
    }

    fn slot_disp(slot: usize, sub_idx: usize) -> i32 {
        -((slot + sub_idx) as i32 * 8)
    }

    fn emit_load_slot_rax(&mut self, slot: usize, sub_idx: usize) {
        let disp = Self::slot_disp(slot, sub_idx);
        self.mc.code.extend_from_slice(&[0x48, 0x8B, 0x85]); // MOV RAX, [RBP + disp32]
        self.mc.code.extend_from_slice(&disp.to_le_bytes());
    }

    fn emit_store_rax_slot(&mut self, slot: usize, sub_idx: usize) {
        let disp = Self::slot_disp(slot, sub_idx);
        self.mc.code.extend_from_slice(&[0x48, 0x89, 0x85]); // MOV [RBP + disp32], RAX
        self.mc.code.extend_from_slice(&disp.to_le_bytes());
    }

    fn emit_lea_rax_slot(&mut self, slot: usize, sub_idx: usize) {
        let disp = Self::slot_disp(slot, sub_idx);
        self.mc.code.extend_from_slice(&[0x48, 0x8D, 0x85]); // LEA RAX, [RBP + disp32]
        self.mc.code.extend_from_slice(&disp.to_le_bytes());
    }

    fn emit_mov_rax_imm64(&mut self, val: i64) {
        if val == 0 {
            self.mc.code.extend_from_slice(&[0x31, 0xC0]); // XOR EAX, EAX
        } else if val >= 0 && val <= i32::MAX as i64 {
            self.mc.code.push(0xB8); // MOV EAX, imm32 (zero-extends to RAX in 5 bytes!)
            self.mc.code.extend_from_slice(&(val as u32).to_le_bytes());
        } else {
            self.mc.code.extend_from_slice(&[0x48, 0xB8]); // MOV RAX, imm64
            self.mc.code.extend_from_slice(&val.to_le_bytes());
        }
    }

    fn emit(&mut self, f: &FnDecl) {
        // Check refinement parameters (`where idx < arr.len`) for zero-cost static bounds elision!
        for p in &f.params {
            if matches!(p.ty, TypeSpec::Refined(_, _)) {
                self.elide_bounds_vars.insert(p.name.clone());
            }
        }

        // Prolog: ENDBR64 + PUSH RBP + MOV RBP, RSP + Win64 4 KB guard-page probe (`__chkstk`)
        self.mc
            .code
            .extend_from_slice(&[0xF3, 0x0F, 0x1E, 0xFA, 0x55, 0x48, 0x89, 0xE5]);
        self.mc.code.push(0xB8); // MOV EAX, <patched_frame_bytes>
        let frame_imm_pos = self.mc.code.len();
        self.mc.code.extend_from_slice(&0u32.to_le_bytes());
        let probe_loop = self.mc.code.len();
        self.mc.code.extend_from_slice(&[0x3D, 0x00, 0x10, 0x00, 0x00]); // CMP EAX, 0x1000
        self.mc.code.push(0x72); // JB .probe_done
        let jb_done_pos = self.mc.code.len();
        self.mc.code.push(0);
        self.mc
            .code
            .extend_from_slice(&[0x48, 0x81, 0xEC, 0x00, 0x10, 0x00, 0x00]); // SUB RSP, 0x1000
        self.mc.code.extend_from_slice(&[0x80, 0x0C, 0x24, 0x00]); // OR BYTE PTR [RSP], 0
        self.mc.code.extend_from_slice(&[0x2D, 0x00, 0x10, 0x00, 0x00]); // SUB EAX, 0x1000
        let jmp_back_end = self.mc.code.len() + 2;
        let back_rel = (probe_loop as isize - jmp_back_end as isize) as i8;
        self.mc.code.extend_from_slice(&[0xEB, back_rel as u8]); // JMP .probe_loop
        let probe_done = self.mc.code.len();
        self.mc.code[jb_done_pos] = (probe_done - (jb_done_pos + 1)) as u8;
        self.mc.code.extend_from_slice(&[0x48, 0x29, 0xC4]); // SUB RSP, RAX

        // Bind parameters into local slots
        if self.meta.use_fast_abi {
            // Fast Register ABI: RCX, RDX, R8, R9
            let reg_stores: [&[u8]; 4] = [
                &[0x48, 0x89, 0x8D], // MOV [RBP+disp32], RCX
                &[0x48, 0x89, 0x95], // MOV [RBP+disp32], RDX
                &[0x4C, 0x89, 0x85], // MOV [RBP+disp32], R8
                &[0x4C, 0x89, 0x8D], // MOV [RBP+disp32], R9
            ];
            for (i, p) in f.params.iter().enumerate() {
                let pty = self.meta.param_tys[i].clone();
                let slot = self.alloc_slots(1);
                let disp = Self::slot_disp(slot, 0);
                self.mc.code.extend_from_slice(reg_stores[i]);
                self.mc.code.extend_from_slice(&disp.to_le_bytes());
                self.vars.insert(p.name.clone(), LocalVar { slot, ty: pty });
            }
        } else {
            // General Struct/Inout ABI: RCX = arg_buf_ptr, RDX = ret_buf_ptr
            let ab_slot = self.alloc_slots(1);
            let rb_slot = self.alloc_slots(1);
            self.arg_buf_ptr_slot = Some(ab_slot);
            self.ret_buf_ptr_slot = Some(rb_slot);
            let d_ab = Self::slot_disp(ab_slot, 0);
            let d_rb = Self::slot_disp(rb_slot, 0);
            self.mc.code.extend_from_slice(&[0x48, 0x89, 0x8D]);
            self.mc.code.extend_from_slice(&d_ab.to_le_bytes());
            self.mc.code.extend_from_slice(&[0x48, 0x89, 0x95]);
            self.mc.code.extend_from_slice(&d_rb.to_le_bytes());

            for (i, p) in f.params.iter().enumerate() {
                let pty = self.meta.param_tys[i].clone();
                let slots = self.mc.type_slots(&pty);
                let local_slot = self.alloc_slots(slots);
                let arg_off = self.meta.param_offsets[i];
                // R8 = arg_buf_ptr
                self.mc.code.extend_from_slice(&[0x4C, 0x8B, 0x85]);
                self.mc.code.extend_from_slice(&d_ab.to_le_bytes());
                for s in 0..slots {
                    let src_byte_off = ((arg_off + s) * 8) as i32;
                    self.mc.code.extend_from_slice(&[0x49, 0x8B, 0x80]); // MOV RAX, [R8 + disp32]
                    self.mc.code.extend_from_slice(&src_byte_off.to_le_bytes());
                    self.emit_store_rax_slot(local_slot, s);
                }
                self.vars.insert(
                    p.name.clone(),
                    LocalVar {
                        slot: local_slot,
                        ty: pty,
                    },
                );
            }
        }

        // Compile body statements with Perceus deterministic Drop scheduling
        for (stmt_idx, stmt) in f.body.iter().enumerate() {
            self.emit_stmt(stmt, f);
            if let Some(drops) = self
                .perceus_plan
                .as_ref()
                .and_then(|p| p.drops_after_stmt.get(&stmt_idx))
                .cloned()
            {
                for (vname, _) in drops {
                    self.emit_perceus_drop_var(&vname);
                }
            }
        }

        // Implicit fallthrough epilogue (returns 0)
        self.emit_perceus_epilogue_drops();
        self.emit_writeback_mut_params(f);
        self.emit_wipe_secrets();
        self.mc.code.extend_from_slice(&[0x31, 0xC0]); // XOR EAX, EAX
        self.mc
            .code
            .extend_from_slice(&[0x48, 0x89, 0xEC, 0x5D, 0xC3]);

        // Patch 16-byte aligned stack frame size (+ 64 bytes Win64 shadow/call scratch!)
        let raw_bytes = (self.next_slot * 8) + 96;
        let aligned_frame = ((raw_bytes + 15) & !15) as u32;
        self.mc.code[frame_imm_pos..frame_imm_pos + 4]
            .copy_from_slice(&aligned_frame.to_le_bytes());
    }

    fn emit_perceus_drop_var(&mut self, var_name: &str) {
        let Some(lv) = self.vars.get(var_name).cloned() else {
            return;
        };
        self.mc.perceus_drops += 1;
        // Load pointer from slot 0 (`Table`, `ByteBuf`, and `IntBuf` all keep heap base ptr in slot 0)
        self.emit_load_slot_rax(lv.slot, 0);
        // TEST RAX, RAX; JZ .skip_free
        self.mc.code.extend_from_slice(&[0x48, 0x85, 0xC0, 0x0F, 0x84]);
        let jz_skip = self.mc.code.len();
        self.mc.code.extend_from_slice(&0i32.to_le_bytes());

        // Zero slot 0 first so early-return / epilogue paths can never double-free
        self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // MOV RCX, RAX (lpAddress)
        self.emit_mov_rax_imm64(0);
        self.emit_store_rax_slot(lv.slot, 0);
        // VirtualFree(RCX, RDX=0, R8D=0x8000 MEM_RELEASE)
        self.mc.code.extend_from_slice(&[0x31, 0xD2]); // XOR EDX, EDX
        self.mc
            .code
            .extend_from_slice(&[0x41, 0xB8, 0x00, 0x80, 0x00, 0x00]); // MOV R8D, 0x8000
        self.mc.emit_call_iat(IAT_VIRTUAL_FREE);

        let after_pos = self.mc.code.len();
        let rel = (after_pos as isize - (jz_skip as isize + 4)) as i32;
        self.mc.code[jz_skip..jz_skip + 4].copy_from_slice(&rel.to_le_bytes());
    }

    fn emit_perceus_epilogue_drops(&mut self) {
        if let Some(drops) = self.perceus_plan.as_ref().map(|p| p.epilogue_drops.clone()) {
            for (vname, _) in drops {
                self.emit_perceus_drop_var(&vname);
            }
        }
    }

    fn emit_writeback_mut_params(&mut self, f: &FnDecl) {
        if let Some(ab_slot) = self.arg_buf_ptr_slot {
            let d_ab = Self::slot_disp(ab_slot, 0);
            for (i, p) in f.params.iter().enumerate() {
                if matches!(p.mode, ParamMode::Mut) {
                    if let Some(lv) = self.vars.get(&p.name).cloned() {
                        let slots = self.mc.type_slots(&lv.ty);
                        let arg_off = self.meta.param_offsets[i];
                        self.mc.code.extend_from_slice(&[0x4C, 0x8B, 0x85]); // MOV R8, [RBP+d_ab]
                        self.mc.code.extend_from_slice(&d_ab.to_le_bytes());
                        for s in 0..slots {
                            self.emit_load_slot_rax(lv.slot, s);
                            let dst_off = ((arg_off + s) * 8) as i32;
                            self.mc.code.extend_from_slice(&[0x49, 0x89, 0x80]); // MOV [R8+disp32], RAX
                            self.mc.code.extend_from_slice(&dst_off.to_le_bytes());
                        }
                    }
                }
            }
        }
    }

    fn emit_wipe_secrets(&mut self) {
        if self.secret_wipe_ranges.is_empty() {
            return;
        }
        self.mc.code.extend_from_slice(&[0x31, 0xC0]); // XOR EAX, EAX
        let ranges = self.secret_wipe_ranges.clone();
        for (base_slot, count) in ranges {
            for s in 0..count {
                self.emit_store_rax_slot(base_slot, s);
            }
        }
    }

    fn emit_return_from_slot(&mut self, val_slot: usize, val_ty: &Ty, f: &FnDecl) {
        self.emit_perceus_epilogue_drops();
        self.emit_writeback_mut_params(f);
        let rslots = self.meta.ret_slots;
        if self.meta.use_fast_abi {
            if rslots >= 2 {
                self.emit_load_slot_rax(val_slot, 1);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC2]); // MOV RDX, RAX
            }
            self.emit_load_slot_rax(val_slot, 0);
        } else if let Some(rb_slot) = self.ret_buf_ptr_slot {
            let d_rb = Self::slot_disp(rb_slot, 0);
            self.mc.code.extend_from_slice(&[0x4C, 0x8B, 0x85]); // MOV R8, [RBP+d_rb]
            self.mc.code.extend_from_slice(&d_rb.to_le_bytes());
            let copy_n = rslots.min(self.mc.type_slots(val_ty).max(1));
            for s in 0..copy_n {
                self.emit_load_slot_rax(val_slot, s);
                let dst_off = (s * 8) as i32;
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0x80]);
                self.mc.code.extend_from_slice(&dst_off.to_le_bytes());
            }
            self.emit_load_slot_rax(val_slot, 0);
        }
        // Preserve RAX/RDX while wiping any `@secret` stack buffers!
        if !self.secret_wipe_ranges.is_empty() {
            self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC1]); // MOV R9, RAX
            self.emit_wipe_secrets();
            self.mc.code.extend_from_slice(&[0x4C, 0x89, 0xC8]); // MOV RAX, R9
        }
        self.mc
            .code
            .extend_from_slice(&[0x48, 0x89, 0xEC, 0x5D, 0xC3]);
    }

    fn emit_stmt(&mut self, stmt: &Stmt, f: &FnDecl) {
        match stmt {
            Stmt::VarDecl { name, ty, init, .. } => {
                // Check Perceus FBIP In-Place Reuse (`fbip_reuse_from`)
                if let Some(donor_name) = self
                    .perceus_plan
                    .as_ref()
                    .and_then(|p| p.fbip_reuse_from.get(name))
                    .cloned()
                {
                    if let Some(donor_lv) = self.vars.get(&donor_name).cloned() {
                        if let Expr::Call { callee, args, .. } = init {
                            if (callee == "buf_new" || callee == "ibuf_new") && args.len() == 1 {
                                self.mc.perceus_reuses += 1;
                                let (cap_slot, _) = self.emit_expr(&args[0].expr, f);
                                let dst_slot = self.alloc_slots(1);

                                // Load donor_ptr into R8 and zero donor slot
                                self.emit_load_slot_rax(donor_lv.slot, 0);
                                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // MOV R8, RAX
                                self.emit_mov_rax_imm64(0);
                                self.emit_store_rax_slot(donor_lv.slot, 0);

                                // Check R8 != 0 && req_cap <= [R8 + 8]
                                self.emit_load_slot_rax(cap_slot, 0);
                                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // MOV RCX, req_cap
                                self.mc.code.extend_from_slice(&[0x4D, 0x85, 0xC0, 0x0F, 0x84]); // TEST R8, R8; JZ .slow_alloc
                                let jz_slow = self.mc.code.len();
                                self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                                // CMP RCX, [R8 + 8]; JA .free_and_alloc
                                self.mc.code.extend_from_slice(&[0x49, 0x3B, 0x48, 0x08, 0x0F, 0x87]);
                                let ja_free = self.mc.code.len();
                                self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                                // FAST IN-PLACE FBIP REUSE: reset `[R8 + 0] = 0` (`len = 0`), RAX = R8 (0 syscalls!)
                                self.mc
                                    .code
                                    .extend_from_slice(&[0x49, 0xC7, 0x00, 0x00, 0x00, 0x00, 0x00]);
                                self.mc.code.extend_from_slice(&[0x4C, 0x89, 0xC0]); // MOV RAX, R8
                                self.mc.code.push(0xE9);
                                let jmp_done = self.mc.code.len();
                                self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                                // .free_and_alloc: VirtualFree(R8, 0, 0x8000)
                                let free_pos = self.mc.code.len();
                                let rel_free = (free_pos as isize - (ja_free as isize + 4)) as i32;
                                self.mc.code[ja_free..ja_free + 4]
                                    .copy_from_slice(&rel_free.to_le_bytes());
                                self.mc.code.extend_from_slice(&[0x4C, 0x89, 0xC1, 0x31, 0xD2]);
                                self.mc
                                    .code
                                    .extend_from_slice(&[0x41, 0xB8, 0x00, 0x80, 0x00, 0x00]);
                                self.mc.emit_call_iat(IAT_VIRTUAL_FREE);

                                // .slow_alloc: fallback fresh allocation
                                let slow_pos = self.mc.code.len();
                                let rel_slow = (slow_pos as isize - (jz_slow as isize + 4)) as i32;
                                self.mc.code[jz_slow..jz_slow + 4]
                                    .copy_from_slice(&rel_slow.to_le_bytes());
                                let (fresh_slot, _) = self.emit_expr(init, f);
                                self.emit_load_slot_rax(fresh_slot, 0);

                                let done_pos = self.mc.code.len();
                                let rel_done = (done_pos as isize - (jmp_done as isize + 4)) as i32;
                                self.mc.code[jmp_done..jmp_done + 4]
                                    .copy_from_slice(&rel_done.to_le_bytes());

                                self.emit_store_rax_slot(dst_slot, 0);
                                let final_ty = Ty::Struct(if callee == "buf_new" {
                                    "ByteBuf".into()
                                } else {
                                    "IntBuf".into()
                                });
                                self.vars.insert(
                                    name.clone(),
                                    LocalVar {
                                        slot: dst_slot,
                                        ty: final_ty,
                                    },
                                );
                                return;
                            }
                        } else if let Expr::TableAlloc { elem_ty, cap, .. } = init {
                            self.mc.perceus_reuses += 1;
                            let (cap_slot, _) = self.emit_expr(cap, f);
                            let dst_slot = self.alloc_slots(2);

                            // Load donor_ptr (R8) and donor_cap (R9), zero donor slot
                            self.emit_load_slot_rax(donor_lv.slot, 0);
                            self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // MOV R8, RAX
                            self.emit_load_slot_rax(donor_lv.slot, 1);
                            self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC1]); // MOV R9, RAX
                            self.emit_mov_rax_imm64(0);
                            self.emit_store_rax_slot(donor_lv.slot, 0);

                            self.emit_load_slot_rax(cap_slot, 0);
                            self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // MOV RCX, req_cap
                            self.mc.code.extend_from_slice(&[0x4D, 0x85, 0xC0, 0x0F, 0x84]); // TEST R8, R8; JZ .slow
                            let jz_slow = self.mc.code.len();
                            self.mc.code.extend_from_slice(&0i32.to_le_bytes());
                            self.mc.code.extend_from_slice(&[0x4C, 0x39, 0xC9, 0x0F, 0x87]); // CMP RCX, R9; JA .free
                            let ja_free = self.mc.code.len();
                            self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                            // Reuse donor Table memory block in-place (0 syscalls!)
                            self.mc.code.extend_from_slice(&[0x4C, 0x89, 0xC0]); // MOV RAX, R8
                            self.mc.code.push(0xE9);
                            let jmp_done = self.mc.code.len();
                            self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                            let free_pos = self.mc.code.len();
                            let rel_free = (free_pos as isize - (ja_free as isize + 4)) as i32;
                            self.mc.code[ja_free..ja_free + 4]
                                .copy_from_slice(&rel_free.to_le_bytes());
                            self.mc.code.extend_from_slice(&[0x4C, 0x89, 0xC1, 0x31, 0xD2]);
                            self.mc
                                .code
                                .extend_from_slice(&[0x41, 0xB8, 0x00, 0x80, 0x00, 0x00]);
                            self.mc.emit_call_iat(IAT_VIRTUAL_FREE);

                            let slow_pos = self.mc.code.len();
                            let rel_slow = (slow_pos as isize - (jz_slow as isize + 4)) as i32;
                            self.mc.code[jz_slow..jz_slow + 4]
                                .copy_from_slice(&rel_slow.to_le_bytes());
                            let (fresh_slot, _) = self.emit_expr(init, f);
                            self.emit_load_slot_rax(fresh_slot, 0);

                            let done_pos = self.mc.code.len();
                            let rel_done = (done_pos as isize - (jmp_done as isize + 4)) as i32;
                            self.mc.code[jmp_done..jmp_done + 4]
                                .copy_from_slice(&rel_done.to_le_bytes());

                            self.emit_store_rax_slot(dst_slot, 0);
                            self.emit_load_slot_rax(cap_slot, 0);
                            self.emit_store_rax_slot(dst_slot, 1);
                            self.vars.insert(
                                name.clone(),
                                LocalVar {
                                    slot: dst_slot,
                                    ty: Ty::Table(elem_ty.clone()),
                                },
                            );
                            return;
                        }
                    }
                }

                let (init_slot, inferred_ty) = self.emit_expr(init, f);
                let final_ty = ty
                    .as_ref()
                    .map(|ts| {
                        resolve_typespec(ts, &self.mc.checked.structs, &self.mc.checked.interfaces)
                    })
                    .unwrap_or(inferred_ty);
                let slots = self.mc.type_slots(&final_ty);
                let dst_slot = self.alloc_slots(slots);
                for s in 0..slots {
                    self.emit_load_slot_rax(init_slot, s);
                    self.emit_store_rax_slot(dst_slot, s);
                }
                self.vars.insert(
                    name.clone(),
                    LocalVar {
                        slot: dst_slot,
                        ty: final_ty,
                    },
                );
            }
            Stmt::TupleDestruct { names, init, .. } => {
                let (init_slot, init_ty) = self.emit_expr(init, f);
                let elem_tys = match init_ty {
                    Ty::Tuple(v) => v,
                    Ty::Struct(sname) => self
                        .mc
                        .checked
                        .structs
                        .get(&sname)
                        .map(|l| l.fields.iter().map(|x| x.1.clone()).collect())
                        .unwrap_or_else(|| vec![Ty::Int; names.len()]),
                    _ => vec![Ty::Int; names.len()],
                };
                let mut cur_off = 0usize;
                for (i, n) in names.iter().enumerate() {
                    let ety = elem_tys.get(i).cloned().unwrap_or(Ty::Int);
                    let eslots = self.mc.type_slots(&ety);
                    let vslot = self.alloc_slots(eslots);
                    for s in 0..eslots {
                        self.emit_load_slot_rax(init_slot, cur_off + s);
                        self.emit_store_rax_slot(vslot, s);
                    }
                    cur_off += eslots;
                    self.vars.insert(n.clone(), LocalVar { slot: vslot, ty: ety });
                }
            }
            Stmt::Assign {
                target, op, val, ..
            } => {
                if let Some((base_slot, sub_off, lval_ty)) = self.resolve_lvalue_slot(target) {
                    let (rhs_slot, _) = self.emit_expr(val, f);
                    let slots = self.mc.type_slots(&lval_ty);
                    if *op == AssignOp::Eq {
                        for s in 0..slots {
                            self.emit_load_slot_rax(rhs_slot, s);
                            self.emit_store_rax_slot(base_slot, sub_off + s);
                        }
                    } else {
                        let bin_op = match op {
                            AssignOp::AddEq => BinOp::Add,
                            AssignOp::SubEq => BinOp::Sub,
                            AssignOp::MulEq => BinOp::Mul,
                            AssignOp::DivEq => BinOp::Div,
                            AssignOp::Eq => unreachable!(),
                        };
                        let res_slot =
                            self.emit_scalar_binop(bin_op, base_slot, sub_off, rhs_slot, 0, &lval_ty);
                        self.emit_load_slot_rax(res_slot, 0);
                        self.emit_store_rax_slot(base_slot, sub_off);
                    }
                } else if let Expr::Index { base, idx, .. } = target {
                    // Table[i64]/Table[f32] indexed write: `table[idx] = val` with bounds check!
                    let (tbl_slot, tbl_ty) = self.emit_expr(base, f);
                    let (idx_slot, _) = self.emit_expr(idx, f);
                    let (rhs_slot, _) = self.emit_expr(val, f);
                    self.emit_bounds_check(idx_slot, tbl_slot, idx);
                    // R8 = base_ptr, RCX = idx, RAX = val
                    self.emit_load_slot_rax(tbl_slot, 0);
                    self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // MOV R8, RAX
                    self.emit_load_slot_rax(idx_slot, 0);
                    self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // MOV RCX, RAX
                    self.emit_load_slot_rax(rhs_slot, 0);
                    if matches!(tbl_ty, Ty::Table(ref e) if e == "f32") {
                        // Convert f64 bits in RAX -> f32 in XMM0 -> [R8 + RCX*4]
                        self.mc
                            .code
                            .extend_from_slice(&[0x66, 0x48, 0x0F, 0x6E, 0xC0]); // MOVQ XMM0, RAX
                        self.mc.code.extend_from_slice(&[0xF2, 0x0F, 0x5A, 0xC0]); // CVTSD2SS XMM0, XMM0
                        self.mc
                            .code
                            .extend_from_slice(&[0xF3, 0x41, 0x0F, 0x11, 0x04, 0x88]); // MOVSS [R8+RCX*4], XMM0
                    } else {
                        self.mc.code.extend_from_slice(&[0x49, 0x89, 0x04, 0xC8]); // MOV [R8+RCX*8], RAX
                    }
                }
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
                ..
            } => {
                let (cslot, _) = self.emit_expr(cond, f);
                self.emit_load_slot_rax(cslot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x85, 0xC0]); // TEST RAX, RAX
                self.mc.code.extend_from_slice(&[0x0F, 0x84]); // JZ else_label
                let jz_pos = self.mc.code.len();
                self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                for s in then_body {
                    self.emit_stmt(s, f);
                }

                if else_body.is_empty() {
                    let after = self.mc.code.len();
                    let rel = (after as isize - (jz_pos as isize + 4)) as i32;
                    self.mc.code[jz_pos..jz_pos + 4].copy_from_slice(&rel.to_le_bytes());
                } else {
                    self.mc.code.push(0xE9); // JMP end_if
                    let jmp_end_pos = self.mc.code.len();
                    self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                    let else_start = self.mc.code.len();
                    let rel_else = (else_start as isize - (jz_pos as isize + 4)) as i32;
                    self.mc.code[jz_pos..jz_pos + 4].copy_from_slice(&rel_else.to_le_bytes());

                    for s in else_body {
                        self.emit_stmt(s, f);
                    }
                    let end_if = self.mc.code.len();
                    let rel_end = (end_if as isize - (jmp_end_pos as isize + 4)) as i32;
                    self.mc.code[jmp_end_pos..jmp_end_pos + 4]
                        .copy_from_slice(&rel_end.to_le_bytes());
                }
            }
            Stmt::While { cond, body, .. } => {
                let loop_top = self.mc.code.len();
                let (cslot, _) = self.emit_expr(cond, f);
                self.emit_load_slot_rax(cslot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x85, 0xC0]);
                self.mc.code.extend_from_slice(&[0x0F, 0x84]); // JZ loop_exit
                let jz_exit = self.mc.code.len();
                self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                self.loop_stack.push(LoopPatch {
                    continue_target: loop_top,
                    break_jumps: Vec::new(),
                });
                for s in body {
                    self.emit_stmt(s, f);
                }
                let lp = self.loop_stack.pop().unwrap();

                self.mc.code.push(0xE9); // JMP loop_top
                let jmp_back = self.mc.code.len();
                self.mc.code.extend_from_slice(&0i32.to_le_bytes());
                let rel_back = (loop_top as isize - (jmp_back as isize + 4)) as i32;
                self.mc.code[jmp_back..jmp_back + 4].copy_from_slice(&rel_back.to_le_bytes());

                let loop_exit = self.mc.code.len();
                let rel_exit = (loop_exit as isize - (jz_exit as isize + 4)) as i32;
                self.mc.code[jz_exit..jz_exit + 4].copy_from_slice(&rel_exit.to_le_bytes());
                for bpos in lp.break_jumps {
                    let r = (loop_exit as isize - (bpos as isize + 4)) as i32;
                    self.mc.code[bpos..bpos + 4].copy_from_slice(&r.to_le_bytes());
                }
            }
            Stmt::ForRange {
                var,
                start,
                end,
                body,
                ..
            } => {
                let (s_slot, _) = self.emit_expr(start, f);
                let (e_slot, _) = self.emit_expr(end, f);
                let iv_slot = self.alloc_slots(1);
                self.emit_load_slot_rax(s_slot, 0);
                self.emit_store_rax_slot(iv_slot, 0);
                self.vars.insert(
                    var.clone(),
                    LocalVar {
                        slot: iv_slot,
                        ty: Ty::Int,
                    },
                );
                // If `end` is `arr.len`, mark `var` as statically proven in-bounds!
                let was_elided = if matches!(end, Expr::Field { field, .. } if field == "len") {
                    self.elide_bounds_vars.insert(var.clone());
                    true
                } else {
                    false
                };

                let loop_top = self.mc.code.len();
                self.emit_load_slot_rax(iv_slot, 0);
                let d_end = Self::slot_disp(e_slot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x3B, 0x85]); // CMP RAX, [RBP+d_end]
                self.mc.code.extend_from_slice(&d_end.to_le_bytes());
                self.mc.code.extend_from_slice(&[0x0F, 0x8D]); // JGE exit
                let jge_exit = self.mc.code.len();
                self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                for s in body {
                    self.emit_stmt(s, f);
                }

                // iv += 1
                self.emit_load_slot_rax(iv_slot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0xFF, 0xC0]); // INC RAX
                self.emit_store_rax_slot(iv_slot, 0);

                self.mc.code.push(0xE9);
                let jb = self.mc.code.len();
                self.mc.code.extend_from_slice(&0i32.to_le_bytes());
                let rel_back = (loop_top as isize - (jb as isize + 4)) as i32;
                self.mc.code[jb..jb + 4].copy_from_slice(&rel_back.to_le_bytes());

                let exit_pos = self.mc.code.len();
                let rel_exit = (exit_pos as isize - (jge_exit as isize + 4)) as i32;
                self.mc.code[jge_exit..jge_exit + 4].copy_from_slice(&rel_exit.to_le_bytes());
                if was_elided {
                    self.elide_bounds_vars.remove(var);
                }
            }
            Stmt::ForEach {
                var,
                collection,
                body,
                ..
            } => {
                self.emit_soa_simd_foreach(var, collection, body, f);
            }
            Stmt::With {
                var, init, body, ..
            } => {
                let (islot, ity) = self.emit_expr(init, f);
                let slots = self.mc.type_slots(&ity);
                let vslot = self.alloc_slots(slots);
                for s in 0..slots {
                    self.emit_load_slot_rax(islot, s);
                    self.emit_store_rax_slot(vslot, s);
                }
                self.vars
                    .insert(var.clone(), LocalVar { slot: vslot, ty: ity });
                for s in body {
                    self.emit_stmt(s, f);
                }
            }
            Stmt::Return { val, .. } => {
                if let Some(e) = val {
                    let (vslot, vty) = self.emit_expr(e, f);
                    self.emit_return_from_slot(vslot, &vty, f);
                } else {
                    let z = self.alloc_slots(1);
                    self.emit_mov_rax_imm64(0);
                    self.emit_store_rax_slot(z, 0);
                    self.emit_return_from_slot(z, &Ty::Unit, f);
                }
            }
            Stmt::Break { .. } => {
                if let Some(lp) = self.loop_stack.last_mut() {
                    self.mc.code.push(0xE9);
                    let bpos = self.mc.code.len();
                    self.mc.code.extend_from_slice(&0i32.to_le_bytes());
                    lp.break_jumps.push(bpos);
                }
            }
            Stmt::Continue { .. } => {
                if let Some(lp) = self.loop_stack.last() {
                    let target = lp.continue_target;
                    self.mc.code.push(0xE9);
                    let cpos = self.mc.code.len();
                    self.mc.code.extend_from_slice(&0i32.to_le_bytes());
                    let rel = (target as isize - (cpos as isize + 4)) as i32;
                    self.mc.code[cpos..cpos + 4].copy_from_slice(&rel.to_le_bytes());
                }
            }
            Stmt::ExprStmt { expr, .. } => {
                let _ = self.emit_expr(expr, f);
            }
        }
    }

    /// Emits Genuine Hardware AVX2 + FMA3 256-Bit Columnar SIMD Machine Code for `for p in table`!
    fn emit_soa_simd_foreach(
        &mut self,
        var: &str,
        collection: &Expr,
        body: &[Stmt],
        f: &FnDecl,
    ) {
        let (tbl_slot, tbl_ty) = self.emit_expr(collection, f);
        let struct_name = match tbl_ty {
            Ty::Table(s) => s,
            _ => return,
        };
        let Some(plan) =
            analyze_foreach_simd(var, &struct_name, body, &self.mc.checked.structs)
        else {
            return;
        };

        self.mc.dead_cols_eliminated += plan.eliminated_dead_columns;

        for op in plan.ops {
            let (dst_col, src_col_opt, scale_slot) = match op {
                SimdKernelOp::Fma {
                    dst_col,
                    src_col,
                    scale_expr,
                } => {
                    let (dt_slot, _) = self.emit_expr(&scale_expr, f);
                    (dst_col, Some(src_col), Some(dt_slot))
                }
                SimdKernelOp::AddCol { dst_col, src_col } => (dst_col, Some(src_col), None),
                SimdKernelOp::ScaleCol {
                    dst_col,
                    scale_expr,
                } => {
                    let (dt_slot, _) = self.emit_expr(&scale_expr, f);
                    (dst_col, None, Some(dt_slot))
                }
            };

            self.mc.simd_loops_emitted += 1;
            self.mc.code.extend_from_slice(&[0x56, 0x57]); // PUSH RSI; PUSH RDI

            // Broadcast scalar factor (or `1.0f32`) into YMM0 (8 x f32)
            if let Some(dt_s) = scale_slot {
                self.emit_load_slot_rax(dt_s, 0);
                self.mc
                    .code
                    .extend_from_slice(&[0x66, 0x48, 0x0F, 0x6E, 0xC0]); // MOVQ XMM0, RAX
                self.mc
                    .code
                    .extend_from_slice(&[0xF2, 0x0F, 0x5A, 0xC0]); // CVTSD2SS XMM0, XMM0
            } else {
                self.mc
                    .code
                    .extend_from_slice(&[0xB8, 0x00, 0x00, 0x80, 0x3F]); // 1.0f32
                self.mc.code.extend_from_slice(&[0x66, 0x0F, 0x6E, 0xC0]); // MOVD XMM0, EAX
            }
            // VBROADCASTSS YMM0, XMM0
            self.mc
                .code
                .extend_from_slice(&[0xC4, 0xE2, 0x7D, 0x18, 0xC0]);

            // R8 = base_ptr, RCX = col_bytes (cap * 4)
            self.emit_load_slot_rax(tbl_slot, 0);
            self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // MOV R8, RAX
            self.emit_load_slot_rax(tbl_slot, 1);
            self.mc
                .code
                .extend_from_slice(&[0x48, 0x89, 0xC1, 0x48, 0xC1, 0xE1, 0x02]); // MOV RCX, RAX; SHL RCX, 2

            if let Some(src_col) = src_col_opt {
                // RSI = base + src_col * col_bytes
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC8]);
                self.mc.code.extend_from_slice(&[0x48, 0x69, 0xC0]);
                self.mc
                    .code
                    .extend_from_slice(&(src_col as i32).to_le_bytes());
                self.mc
                    .code
                    .extend_from_slice(&[0x4C, 0x01, 0xC0, 0x48, 0x89, 0xC6]); // ADD RAX, R8; MOV RSI, RAX
            }

            // RDI = base + dst_col * col_bytes
            self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC8]);
            self.mc.code.extend_from_slice(&[0x48, 0x69, 0xC0]);
            self.mc
                .code
                .extend_from_slice(&(dst_col as i32).to_le_bytes());
            self.mc
                .code
                .extend_from_slice(&[0x4C, 0x01, 0xC0, 0x48, 0x89, 0xC7]); // ADD RAX, R8; MOV RDI, RAX

            // XOR EAX, EAX; TEST RCX, RCX; JZ .done
            self.mc.code.extend_from_slice(&[0x31, 0xC0, 0x48, 0x85, 0xC9, 0x74]);
            let jz_done_pos = self.mc.code.len();
            self.mc.code.push(0);

            let simd_top = self.mc.code.len();
            if src_col_opt.is_some() {
                // VMOVUPS YMM1, [RSI + RAX]; VMOVUPS YMM2, [RDI + RAX]; VFMADD231PS YMM2, YMM1, YMM0
                self.mc
                    .code
                    .extend_from_slice(&[0xC5, 0xFC, 0x10, 0x0C, 0x06]);
                self.mc
                    .code
                    .extend_from_slice(&[0xC5, 0xFC, 0x10, 0x14, 0x07]);
                self.mc
                    .code
                    .extend_from_slice(&[0xC4, 0xE2, 0x75, 0xB8, 0xD0]);
            } else {
                // ScaleCol: VMOVUPS YMM2, [RDI + RAX]; VMULPS YMM2, YMM2, YMM0
                self.mc
                    .code
                    .extend_from_slice(&[0xC5, 0xFC, 0x10, 0x14, 0x07]);
                self.mc
                    .code
                    .extend_from_slice(&[0xC5, 0xEC, 0x59, 0xD0]);
            }
            // VMOVUPS [RDI + RAX], YMM2
            self.mc
                .code
                .extend_from_slice(&[0xC5, 0xFC, 0x11, 0x14, 0x07]);
            self.mc.code.extend_from_slice(&[0x48, 0x83, 0xC0, 0x20]); // ADD RAX, 32
            self.mc.code.extend_from_slice(&[0x48, 0x39, 0xC8, 0x72]); // CMP RAX, RCX; JB simd_top
            let rel = (simd_top as isize - (self.mc.code.len() as isize + 1)) as i8;
            self.mc.code.push(rel as u8);

            let done_pos = self.mc.code.len();
            self.mc.code[jz_done_pos] = (done_pos - (jz_done_pos + 1)) as u8;

            // VZEROUPPER; POP RDI; POP RSI
            self.mc
                .code
                .extend_from_slice(&[0xC5, 0xF8, 0x77, 0x5F, 0x5E]);
        }
    }

    fn emit_bounds_check(&mut self, idx_slot: usize, tbl_slot: usize, idx_expr: &Expr) {
        if let Expr::Ident(vname, _) = idx_expr {
            if self.elide_bounds_vars.contains(vname) {
                // Proven safe by refinement `where idx < arr.len` or `for i in 0..arr.len`!
                return;
            }
        }
        self.emit_load_slot_rax(idx_slot, 0);
        let d_len = Self::slot_disp(tbl_slot, 1);
        self.mc.code.extend_from_slice(&[0x48, 0x3B, 0x85]); // CMP RAX, [RBP + d_len]
        self.mc.code.extend_from_slice(&d_len.to_le_bytes());
        self.mc.emit_jmp_fn(0x83, HELPER_SEC_TRAP); // JAE (unsigned >= catches < 0 AND >= len in 1 branch!)
    }

    fn resolve_lvalue_slot(&self, expr: &Expr) -> Option<(usize, usize, Ty)> {
        match expr {
            Expr::Ident(name, _) => {
                let lv = self.vars.get(name)?;
                Some((lv.slot, 0, lv.ty.clone()))
            }
            Expr::Field { base, field, .. } => {
                let (bslot, boff, bty) = self.resolve_lvalue_slot(base)?;
                if let Ty::Struct(sname) = bty {
                    let (foff, fty) = self.mc.struct_field_slot_and_ty(&sname, field);
                    Some((bslot, boff + foff, fty))
                } else {
                    None
                }
            }
            Expr::TupleIdx { base, idx, .. } => {
                let (bslot, boff, bty) = self.resolve_lvalue_slot(base)?;
                if let Ty::Tuple(elems) = bty {
                    let mut off = 0;
                    for (i, et) in elems.iter().enumerate() {
                        if i == *idx {
                            return Some((bslot, boff + off, et.clone()));
                        }
                        off += self.mc.type_slots(et);
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn emit_expr(&mut self, expr: &Expr, f: &FnDecl) -> (usize, Ty) {
        match expr {
            Expr::Int(v, _) => {
                let s = self.alloc_slots(1);
                self.emit_mov_rax_imm64(*v);
                self.emit_store_rax_slot(s, 0);
                (s, Ty::Int)
            }
            Expr::Float(fv, _) => {
                let s = self.alloc_slots(1);
                self.emit_mov_rax_imm64(fv.to_bits() as i64);
                self.emit_store_rax_slot(s, 0);
                (s, Ty::Float)
            }
            Expr::Dec(dv, _) => {
                let s = self.alloc_slots(1);
                self.emit_mov_rax_imm64(*dv);
                self.emit_store_rax_slot(s, 0);
                (s, Ty::Dec)
            }
            Expr::Bool(bv, _) => {
                let s = self.alloc_slots(1);
                self.emit_mov_rax_imm64(if *bv { 1 } else { 0 });
                self.emit_store_rax_slot(s, 0);
                (s, Ty::Bool)
            }
            Expr::Str(st, _) => {
                let (roff, slen) = self.mc.intern_str(st);
                let s = self.alloc_slots(2);
                self.mc.emit_lea_rax_rdata(roff);
                self.emit_store_rax_slot(s, 0);
                self.emit_mov_rax_imm64(slen as i64);
                self.emit_store_rax_slot(s, 1);
                (s, Ty::Str)
            }
            Expr::SecretStr(plain, _) => {
                // Polymorphic compile-time encryption -> stack-only runtime decryption + auto zeroize!
                let (cipher_off, slen, key) = self.mc.add_encrypted_secret(plain);
                let buf_slots = (slen + 8) / 8;
                let buf_slot = self.alloc_slots(buf_slots.max(1));
                // Note: stack slots grow downward `[RBP - (buf_slot + buf_slots - 1)*8 .. RBP - buf_slot*8]`
                let lowest_slot = buf_slot + buf_slots.max(1) - 1;
                self.secret_wipe_ranges.push((buf_slot, buf_slots.max(1)));

                // R8 = LEA cipher in .rdata, R9 = LEA stack_buf
                self.mc.emit_lea_rax_rdata(cipher_off);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // MOV R8, RAX
                self.emit_lea_rax_slot(lowest_slot, 0);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC1]); // MOV R9, RAX

                // Decrypt each byte directly onto stack buffer
                for i in 0..slen {
                    let ks = (key.rotate_left(((i as u32) * 7) & 63) as u8)
                        .wrapping_add((i as u8).wrapping_mul(37));
                    let ioff = i as i32;
                    // MOVZX EAX, BYTE PTR [R8 + ioff]
                    self.mc.code.extend_from_slice(&[0x41, 0x0F, 0xB6, 0x80]);
                    self.mc.code.extend_from_slice(&ioff.to_le_bytes());
                    // XOR AL, ks
                    self.mc.code.extend_from_slice(&[0x34, ks]);
                    // MOV BYTE PTR [R9 + ioff], AL
                    self.mc.code.extend_from_slice(&[0x41, 0x88, 0x81]);
                    self.mc.code.extend_from_slice(&ioff.to_le_bytes());
                }

                let res = self.alloc_slots(2);
                self.mc.code.extend_from_slice(&[0x4C, 0x89, 0xC8]); // MOV RAX, R9
                self.emit_store_rax_slot(res, 0);
                self.emit_mov_rax_imm64(slen as i64);
                self.emit_store_rax_slot(res, 1);
                (res, Ty::Str)
            }
            Expr::FStr(parts, _) => {
                // FStr used outside `print`/`println` evaluates parts and returns length/string
                let s = self.alloc_slots(2);
                self.emit_mov_rax_imm64(0);
                self.emit_store_rax_slot(s, 0);
                self.emit_mov_rax_imm64(parts.len() as i64);
                self.emit_store_rax_slot(s, 1);
                (s, Ty::Str)
            }
            Expr::Ident(name, _) | Expr::MoveIdent(name, _) => {
                if let Some(lv) = self.vars.get(name).cloned() {
                    (lv.slot, lv.ty)
                } else {
                    let s = self.alloc_slots(1);
                    self.emit_mov_rax_imm64(0);
                    self.emit_store_rax_slot(s, 0);
                    (s, Ty::Int)
                }
            }
            Expr::Tuple(elems, _) => {
                let mut evaled = Vec::new();
                let mut etys = Vec::new();
                let mut total = 0usize;
                for e in elems {
                    let (es, et) = self.emit_expr(e, f);
                    let cnt = self.mc.type_slots(&et);
                    total += cnt;
                    evaled.push((es, cnt));
                    etys.push(et);
                }
                let dst = self.alloc_slots(total.max(1));
                let mut cur = 0;
                for (es, cnt) in evaled {
                    for k in 0..cnt {
                        self.emit_load_slot_rax(es, k);
                        self.emit_store_rax_slot(dst, cur + k);
                    }
                    cur += cnt;
                }
                (dst, Ty::Tuple(etys))
            }
            Expr::Unary { op, expr: inner, .. } => {
                let (islot, ity) = self.emit_expr(inner, f);
                let dst = self.alloc_slots(1);
                self.emit_load_slot_rax(islot, 0);
                match op {
                    UnaryOp::Neg => {
                        if ity == Ty::Float {
                            self.mc.code.extend_from_slice(&[0x48, 0xB9]);
                            self.mc
                                .code
                                .extend_from_slice(&(0x8000_0000_0000_0000u64).to_le_bytes());
                            self.mc.code.extend_from_slice(&[0x48, 0x31, 0xC8]); // XOR sign bit
                        } else {
                            self.mc.code.extend_from_slice(&[0x48, 0xF7, 0xD8]); // NEG RAX
                        }
                        self.emit_store_rax_slot(dst, 0);
                        (dst, ity)
                    }
                    UnaryOp::Not => {
                        self.mc
                            .code
                            .extend_from_slice(&[0x48, 0x85, 0xC0, 0x0F, 0x94, 0xC0, 0x0F, 0xB6, 0xC0]);
                        self.emit_store_rax_slot(dst, 0);
                        (dst, Ty::Bool)
                    }
                }
            }
            Expr::Binary {
                op, left, right, ..
            } => {
                let (lslot, lty) = self.emit_expr(left, f);
                // Result `T!E` Zero-Ceremony Fallback: `res or fallback_expr`!
                if *op == BinOp::Or {
                    if let Ty::Result(ok_ty, _) = lty {
                        let ok_slots = self.mc.type_slots(&ok_ty);
                        let dst = self.alloc_slots(ok_slots);
                        // Check err_tag at slot 0 (`0` == OK!)
                        self.emit_load_slot_rax(lslot, 0);
                        self.mc.code.extend_from_slice(&[0x48, 0x85, 0xC0]); // TEST RAX, RAX
                        self.mc.code.extend_from_slice(&[0x0F, 0x85]); // JNZ .on_err
                        let jnz_err = self.mc.code.len();
                        self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                        // OK path: copy payload from `lslot + 1..`
                        for s in 0..ok_slots {
                            self.emit_load_slot_rax(lslot, 1 + s);
                            self.emit_store_rax_slot(dst, s);
                        }
                        self.mc.code.push(0xE9);
                        let jmp_done = self.mc.code.len();
                        self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                        // Err path: evaluate fallback `right`!
                        let err_pos = self.mc.code.len();
                        let rel_err = (err_pos as isize - (jnz_err as isize + 4)) as i32;
                        self.mc.code[jnz_err..jnz_err + 4].copy_from_slice(&rel_err.to_le_bytes());
                        let (rslot, _) = self.emit_expr(right, f);
                        for s in 0..ok_slots {
                            self.emit_load_slot_rax(rslot, s);
                            self.emit_store_rax_slot(dst, s);
                        }

                        let done_pos = self.mc.code.len();
                        let rel_done = (done_pos as isize - (jmp_done as isize + 4)) as i32;
                        self.mc.code[jmp_done..jmp_done + 4]
                            .copy_from_slice(&rel_done.to_le_bytes());
                        return (dst, *ok_ty);
                    }
                    // Short-circuit boolean `left or right`!
                    let dst = self.alloc_slots(1);
                    self.emit_load_slot_rax(lslot, 0);
                    self.mc.code.extend_from_slice(&[0x48, 0x85, 0xC0, 0x0F, 0x85]); // TEST RAX, RAX; JNZ .is_true
                    let jnz_true = self.mc.code.len();
                    self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                    let (rslot, _) = self.emit_expr(right, f);
                    self.emit_load_slot_rax(rslot, 0);
                    self.mc
                        .code
                        .extend_from_slice(&[0x48, 0x85, 0xC0, 0x0F, 0x95, 0xC0, 0x0F, 0xB6, 0xC0]);
                    self.emit_store_rax_slot(dst, 0);
                    self.mc.code.push(0xE9);
                    let jmp_done = self.mc.code.len();
                    self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                    let true_pos = self.mc.code.len();
                    let rel_t = (true_pos as isize - (jnz_true as isize + 4)) as i32;
                    self.mc.code[jnz_true..jnz_true + 4].copy_from_slice(&rel_t.to_le_bytes());
                    self.emit_mov_rax_imm64(1);
                    self.emit_store_rax_slot(dst, 0);

                    let done_pos = self.mc.code.len();
                    let rel_d = (done_pos as isize - (jmp_done as isize + 4)) as i32;
                    self.mc.code[jmp_done..jmp_done + 4].copy_from_slice(&rel_d.to_le_bytes());
                    return (dst, Ty::Bool);
                }

                if *op == BinOp::And {
                    // Short-circuit boolean `left and right`!
                    let dst = self.alloc_slots(1);
                    self.emit_load_slot_rax(lslot, 0);
                    self.mc.code.extend_from_slice(&[0x48, 0x85, 0xC0, 0x0F, 0x84]); // TEST RAX, RAX; JZ .is_false
                    let jz_false = self.mc.code.len();
                    self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                    let (rslot, _) = self.emit_expr(right, f);
                    self.emit_load_slot_rax(rslot, 0);
                    self.mc
                        .code
                        .extend_from_slice(&[0x48, 0x85, 0xC0, 0x0F, 0x95, 0xC0, 0x0F, 0xB6, 0xC0]);
                    self.emit_store_rax_slot(dst, 0);
                    self.mc.code.push(0xE9);
                    let jmp_done = self.mc.code.len();
                    self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                    let false_pos = self.mc.code.len();
                    let rel_f = (false_pos as isize - (jz_false as isize + 4)) as i32;
                    self.mc.code[jz_false..jz_false + 4].copy_from_slice(&rel_f.to_le_bytes());
                    self.emit_mov_rax_imm64(0);
                    self.emit_store_rax_slot(dst, 0);

                    let done_pos = self.mc.code.len();
                    let rel_d = (done_pos as isize - (jmp_done as isize + 4)) as i32;
                    self.mc.code[jmp_done..jmp_done + 4].copy_from_slice(&rel_d.to_le_bytes());
                    return (dst, Ty::Bool);
                }

                let (rslot, rty) = self.emit_expr(right, f);
                if lty == Ty::Str && rty == Ty::Str && (*op == BinOp::Eq || *op == BinOp::NotEq) {
                    let dst = self.alloc_slots(1);
                    let da0 = Self::slot_disp(lslot, 0);
                    let da1 = Self::slot_disp(lslot, 1);
                    let db0 = Self::slot_disp(rslot, 0);
                    let db1 = Self::slot_disp(rslot, 1);
                    self.mc.code.extend_from_slice(&[0x48, 0x8B, 0x8D]); // MOV RCX, [lslot+0]
                    self.mc.code.extend_from_slice(&da0.to_le_bytes());
                    self.mc.code.extend_from_slice(&[0x48, 0x8B, 0x95]); // MOV RDX, [lslot+1]
                    self.mc.code.extend_from_slice(&da1.to_le_bytes());
                    self.mc.code.extend_from_slice(&[0x4C, 0x8B, 0x85]); // MOV R8, [rslot+0]
                    self.mc.code.extend_from_slice(&db0.to_le_bytes());
                    self.mc.code.extend_from_slice(&[0x4C, 0x8B, 0x8D]); // MOV R9, [rslot+1]
                    self.mc.code.extend_from_slice(&db1.to_le_bytes());
                    self.mc.emit_call_fn(HELPER_STR_EQ);
                    if *op == BinOp::NotEq {
                        self.mc.code.extend_from_slice(&[0x48, 0x83, 0xF0, 0x01]); // XOR RAX, 1
                    }
                    self.emit_store_rax_slot(dst, 0);
                    return (dst, Ty::Bool);
                }

                let common_ty = if lty == Ty::Dec || rty == Ty::Dec {
                    Ty::Dec
                } else if lty == Ty::Float || rty == Ty::Float {
                    Ty::Float
                } else {
                    Ty::Int
                };

                // Mixed `dec * int` or `int * dec` handling!
                if common_ty == Ty::Dec && (*op == BinOp::Mul || *op == BinOp::Div) && lty != rty {
                    let dst = self.alloc_slots(1);
                    self.emit_load_slot_rax(lslot, 0);
                    let d_r = Self::slot_disp(rslot, 0);
                    self.mc.code.extend_from_slice(&[0x48, 0x8B, 0x8D]); // MOV RCX, [RBP+d_r]
                    self.mc.code.extend_from_slice(&d_r.to_le_bytes());
                    if *op == BinOp::Mul {
                        self.mc.code.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC1]); // IMUL RAX, RCX
                        if self.mc.shield_enabled {
                            self.mc.emit_jmp_fn(0x80, HELPER_SEC_TRAP); // JO trap
                        }
                    } else {
                        self.mc.code.extend_from_slice(&[0x48, 0x85, 0xC9]); // TEST RCX, RCX
                        self.mc.emit_jmp_fn(0x84, HELPER_SEC_TRAP); // JZ div0 trap
                        self.mc.code.extend_from_slice(&[0x48, 0x99, 0x48, 0xF7, 0xF9]); // CQO; IDIV RCX
                    }
                    self.emit_store_rax_slot(dst, 0);
                    return (dst, Ty::Dec);
                }

                let res_ty = match op {
                    BinOp::Eq
                    | BinOp::NotEq
                    | BinOp::Lt
                    | BinOp::LtEq
                    | BinOp::Gt
                    | BinOp::GtEq
                    | BinOp::And
                    | BinOp::Or => Ty::Bool,
                    _ => common_ty.clone(),
                };
                let dst = self.emit_scalar_binop(*op, lslot, 0, rslot, 0, &common_ty);
                (dst, res_ty)
            }
            Expr::OrReturn { lhs, ret_val, .. } => {
                let (lslot, lty) = self.emit_expr(lhs, f);
                if let Ty::Result(ok_ty, _) = lty {
                    let ok_slots = self.mc.type_slots(&ok_ty);
                    let dst = self.alloc_slots(ok_slots);
                    self.emit_load_slot_rax(lslot, 0);
                    self.mc.code.extend_from_slice(&[0x48, 0x85, 0xC0]); // TEST RAX, RAX
                    self.mc.code.extend_from_slice(&[0x0F, 0x84]); // JZ .is_ok
                    let jz_ok = self.mc.code.len();
                    self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                    // Error path: return `ret_val` or propagate error!
                    if let Some(rv) = ret_val {
                        let (rv_slot, rv_ty) = self.emit_expr(rv, f);
                        self.emit_return_from_slot(rv_slot, &rv_ty, f);
                    } else {
                        self.emit_return_from_slot(lslot, &self.meta.ret_ty.clone(), f);
                    }

                    let ok_pos = self.mc.code.len();
                    let rel_ok = (ok_pos as isize - (jz_ok as isize + 4)) as i32;
                    self.mc.code[jz_ok..jz_ok + 4].copy_from_slice(&rel_ok.to_le_bytes());
                    for s in 0..ok_slots {
                        self.emit_load_slot_rax(lslot, 1 + s);
                        self.emit_store_rax_slot(dst, s);
                    }
                    (dst, *ok_ty)
                } else {
                    (lslot, lty)
                }
            }
            Expr::Cast {
                expr: inner, ty, ..
            } => {
                let (islot, ity) = self.emit_expr(inner, f);
                let target_ty =
                    resolve_typespec(ty, &self.mc.checked.structs, &self.mc.checked.interfaces);
                let dst = self.alloc_slots(1);
                self.emit_load_slot_rax(islot, 0);
                if ity == Ty::Int && target_ty == Ty::Float {
                    self.mc
                        .code
                        .extend_from_slice(&[0xF2, 0x48, 0x0F, 0x2A, 0xC0]); // CVTSI2SD XMM0, RAX
                    self.mc
                        .code
                        .extend_from_slice(&[0x66, 0x48, 0x0F, 0x7E, 0xC0]); // MOVQ RAX, XMM0
                } else if ity == Ty::Float && target_ty == Ty::Int {
                    self.mc
                        .code
                        .extend_from_slice(&[0x66, 0x48, 0x0F, 0x6E, 0xC0]); // MOVQ XMM0, RAX
                    self.mc
                        .code
                        .extend_from_slice(&[0xF2, 0x48, 0x0F, 0x2C, 0xC0]); // CVTTSD2SI RAX, XMM0
                }
                self.emit_store_rax_slot(dst, 0);
                (dst, target_ty)
            }
            Expr::Field { base, field, .. } => {
                let (bslot, bty) = self.emit_expr(base, f);
                if field == "len" && (matches!(bty, Ty::Str | Ty::Table(_))) {
                    let dst = self.alloc_slots(1);
                    self.emit_load_slot_rax(bslot, 1);
                    self.emit_store_rax_slot(dst, 0);
                    return (dst, Ty::Int);
                }
                if let Ty::Struct(sname) = bty {
                    let (foff, fty) = self.mc.struct_field_slot_and_ty(&sname, field);
                    let fslots = self.mc.type_slots(&fty);
                    let dst = self.alloc_slots(fslots);
                    for s in 0..fslots {
                        self.emit_load_slot_rax(bslot, foff + s);
                        self.emit_store_rax_slot(dst, s);
                    }
                    return (dst, fty);
                }
                (bslot, Ty::Int)
            }
            Expr::TupleIdx { base, idx, .. } => {
                let (bslot, bty) = self.emit_expr(base, f);
                if let Ty::Tuple(elems) = bty {
                    let mut off = 0;
                    for (i, et) in elems.iter().enumerate() {
                        let cnt = self.mc.type_slots(et);
                        if i == *idx {
                            let dst = self.alloc_slots(cnt);
                            for s in 0..cnt {
                                self.emit_load_slot_rax(bslot, off + s);
                                self.emit_store_rax_slot(dst, s);
                            }
                            return (dst, et.clone());
                        }
                        off += cnt;
                    }
                }
                (bslot, Ty::Int)
            }
            Expr::Index { base, idx, .. } => {
                let (tbl_slot, tbl_ty) = self.emit_expr(base, f);
                let (idx_slot, _) = self.emit_expr(idx, f);
                self.emit_bounds_check(idx_slot, tbl_slot, idx);
                let dst = self.alloc_slots(1);
                self.emit_load_slot_rax(tbl_slot, 0);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // MOV R8, RAX
                self.emit_load_slot_rax(idx_slot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // MOV RCX, RAX
                if matches!(tbl_ty, Ty::Table(ref e) if e == "f32") {
                    // MOVSS XMM0, [R8 + RCX*4]; CVTSS2SD XMM0, XMM0; MOVQ RAX, XMM0
                    self.mc
                        .code
                        .extend_from_slice(&[0xF3, 0x41, 0x0F, 0x10, 0x04, 0x88]);
                    self.mc.code.extend_from_slice(&[0xF3, 0x0F, 0x5A, 0xC0]);
                    self.mc
                        .code
                        .extend_from_slice(&[0x66, 0x48, 0x0F, 0x7E, 0xC0]);
                    self.emit_store_rax_slot(dst, 0);
                    (dst, Ty::Float)
                } else {
                    self.mc.code.extend_from_slice(&[0x49, 0x8B, 0x04, 0xC8]); // MOV RAX, [R8+RCX*8]
                    self.emit_store_rax_slot(dst, 0);
                    (dst, Ty::Int)
                }
            }
            Expr::TableAlloc { elem_ty, cap, .. } => {
                // Allocate dense 64B-padded Columnar SoA memory via OS `VirtualAlloc`!
                let (cap_slot, _) = self.emit_expr(cap, f);
                let col_count = self
                    .mc
                    .checked
                    .structs
                    .get(elem_ty)
                    .map(|l| l.fields.len().max(1))
                    .unwrap_or(1);
                let elem_bytes = if elem_ty == "i64" || elem_ty == "usize" || elem_ty == "f64" {
                    8usize
                } else {
                    4usize
                };
                let stride = (col_count * elem_bytes) as i32;

                // RDX = cap * stride + 256 (padded for 8-wide AVX2 alignment!)
                self.emit_load_slot_rax(cap_slot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x69, 0xD0]); // IMUL RDX, RAX, stride
                self.mc.code.extend_from_slice(&stride.to_le_bytes());
                self.mc
                    .code
                    .extend_from_slice(&[0x48, 0x81, 0xC2, 0x00, 0x01, 0x00, 0x00]); // ADD RDX, 256
                // RCX = 0, R8D = 0x3000 (MEM_COMMIT|MEM_RESERVE), R9D = 0x04 (PAGE_READWRITE)
                self.mc.code.extend_from_slice(&[0x31, 0xC9]);
                self.mc
                    .code
                    .extend_from_slice(&[0x41, 0xB8, 0x00, 0x30, 0x00, 0x00]);
                self.mc
                    .code
                    .extend_from_slice(&[0x41, 0xB9, 0x04, 0x00, 0x00, 0x00]);
                self.mc.emit_call_iat(IAT_VIRTUAL_ALLOC);

                let dst = self.alloc_slots(2);
                self.emit_store_rax_slot(dst, 0); // slot 0: base_ptr
                self.emit_load_slot_rax(cap_slot, 0);
                self.emit_store_rax_slot(dst, 1); // slot 1: len
                (dst, Ty::Table(elem_ty.clone()))
            }
            Expr::Call { callee, args, .. } => self.emit_call_expr(callee, args, f),
            Expr::MethodCall {
                receiver,
                method,
                args,
                ..
            } => {
                if method == "join" {
                    return self.emit_expr(receiver, f);
                }
                let (rslot, rty) = self.emit_expr(receiver, f);
                if let Ty::Struct(sname) = &rty {
                    let mangled = format!("{}__{}", sname, method);
                    let target_name = if self.mc.fn_metas.contains_key(&mangled) {
                        mangled
                    } else {
                        method.clone()
                    };
                    let mut full_args = Vec::with_capacity(args.len() + 1);
                    full_args.push(CallArg {
                        mode: ArgPassMode::ByVal,
                        name: None,
                        expr: (*receiver.clone()),
                    });
                    let _ = rslot;
                    full_args.extend_from_slice(args);
                    return self.emit_call_expr(&target_name, &full_args, f);
                }
                if matches!(rty, Ty::Interface(_)) {
                    let all_structs = self.mc.sorted_struct_names();
                    let mut impls: Vec<(i64, String, String)> = Vec::new();
                    for (idx, sname) in all_structs.iter().enumerate() {
                        let mangled = format!("{}__{}", sname, method);
                        if self.mc.fn_metas.contains_key(&mangled) {
                            impls.push(((idx + 1) as i64, sname.clone(), mangled));
                        }
                    }
                    if let Some((_, _, first_mangled)) = impls.first() {
                        let first_meta = self.mc.fn_metas.get(first_mangled).unwrap().clone();
                        let ret_slots = first_meta.ret_slots;
                        let ret_ty = first_meta.ret_ty;
                        let dst = self.alloc_slots(ret_slots);
                        self.emit_mov_rax_imm64(0);
                        self.emit_store_rax_slot(dst, 0);

                        let mut end_jumps = Vec::new();
                        for (sid, sname, mangled) in impls {
                            self.emit_load_slot_rax(rslot, 0);
                            self.mc.code.extend_from_slice(&[0x48, 0x3D]); // CMP RAX, imm32
                            self.mc.code.extend_from_slice(&(sid as i32).to_le_bytes());
                            self.mc.code.extend_from_slice(&[0x0F, 0x85]); // JNE .next_impl
                            let jne_pos = self.mc.code.len();
                            self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                            let sty = Ty::Struct(sname.clone());
                            let s_slots = self.mc.type_slots(&sty);
                            let unpacked = self.alloc_slots(s_slots);
                            for k in 0..s_slots {
                                self.emit_load_slot_rax(rslot, 1 + k);
                                self.emit_store_rax_slot(unpacked, k);
                            }
                            let tmp_name = format!("__duck_recv_{}_{}", sname, unpacked);
                            self.vars.insert(
                                tmp_name.clone(),
                                LocalVar {
                                    slot: unpacked,
                                    ty: sty,
                                },
                            );
                            let mut full_args = Vec::with_capacity(args.len() + 1);
                            full_args.push(CallArg {
                                mode: ArgPassMode::ByVal,
                                name: None,
                                expr: Expr::Ident(tmp_name.clone(), 0),
                            });
                            full_args.extend_from_slice(args);
                            let (c_res, _) = self.emit_call_expr(&mangled, &full_args, f);
                            self.vars.remove(&tmp_name);

                            for k in 0..ret_slots {
                                self.emit_load_slot_rax(c_res, k);
                                self.emit_store_rax_slot(dst, k);
                            }
                            self.mc.code.push(0xE9); // JMP .done_dispatch
                            let jmp_end = self.mc.code.len();
                            self.mc.code.extend_from_slice(&0i32.to_le_bytes());
                            end_jumps.push(jmp_end);

                            let next_pos = self.mc.code.len();
                            let rel_next = (next_pos as isize - (jne_pos as isize + 4)) as i32;
                            self.mc.code[jne_pos..jne_pos + 4]
                                .copy_from_slice(&rel_next.to_le_bytes());
                        }
                        let done_pos = self.mc.code.len();
                        for jp in end_jumps {
                            let rel_end = (done_pos as isize - (jp as isize + 4)) as i32;
                            self.mc.code[jp..jp + 4].copy_from_slice(&rel_end.to_le_bytes());
                        }
                        return (dst, ret_ty);
                    }
                }
                (rslot, Ty::Int)
            }
            Expr::IfExpr {
                cond,
                then_body,
                then_tail,
                else_body,
                else_tail,
                ..
            } => {
                let (cslot, _) = self.emit_expr(cond, f);
                self.emit_load_slot_rax(cslot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x85, 0xC0]);
                self.mc.code.extend_from_slice(&[0x0F, 0x84]); // JZ else
                let jz_else = self.mc.code.len();
                self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                for s in then_body {
                    self.emit_stmt(s, f);
                }
                let (tslot, tty) = self.emit_expr(then_tail, f);
                let rslots = self.mc.type_slots(&tty);
                let dst = self.alloc_slots(rslots);
                for k in 0..rslots {
                    self.emit_load_slot_rax(tslot, k);
                    self.emit_store_rax_slot(dst, k);
                }
                self.mc.code.push(0xE9);
                let jmp_end = self.mc.code.len();
                self.mc.code.extend_from_slice(&0i32.to_le_bytes());

                let else_pos = self.mc.code.len();
                let rel_else = (else_pos as isize - (jz_else as isize + 4)) as i32;
                self.mc.code[jz_else..jz_else + 4].copy_from_slice(&rel_else.to_le_bytes());
                for s in else_body {
                    self.emit_stmt(s, f);
                }
                let (eslot, _) = self.emit_expr(else_tail, f);
                for k in 0..rslots {
                    self.emit_load_slot_rax(eslot, k);
                    self.emit_store_rax_slot(dst, k);
                }

                let end_pos = self.mc.code.len();
                let rel_end = (end_pos as isize - (jmp_end as isize + 4)) as i32;
                self.mc.code[jmp_end..jmp_end + 4].copy_from_slice(&rel_end.to_le_bytes());
                (dst, tty)
            }
            Expr::Spawn { body, .. } => {
                // Execute deterministic colorless fiber block inline on current OS thread
                let mut res_slot = self.alloc_slots(1);
                let mut res_ty = Ty::Int;
                self.emit_mov_rax_imm64(0);
                self.emit_store_rax_slot(res_slot, 0);
                for s in body {
                    if let Stmt::Return { val: Some(rv), .. } = s {
                        let (rs, rt) = self.emit_expr(rv, f);
                        res_slot = rs;
                        res_ty = rt;
                        break;
                    } else {
                        self.emit_stmt(s, f);
                    }
                }
                (res_slot, Ty::Job(Box::new(res_ty)))
            }
        }
    }

    fn emit_call_expr(&mut self, callee: &str, args: &[CallArg], f: &FnDecl) -> (usize, Ty) {
        // 1. Built-in `print` & `println` (Zero format-string vulnerabilities!)
        if callee == "print" || callee == "println" {
            for a in args {
                if let Expr::FStr(parts, _) = &a.expr {
                    for part in parts {
                        match part {
                            FStrPart::Lit(lit) => {
                                if !lit.is_empty() {
                                    let (roff, slen) = self.mc.intern_str(lit);
                                    self.mc.emit_lea_rax_rdata(roff);
                                    self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]);
                                    self.mc.code.push(0xBA);
                                    self.mc
                                        .code
                                        .extend_from_slice(&(slen as u32).to_le_bytes());
                                    self.mc.emit_call_fn(HELPER_PRINT_BYTES);
                                }
                            }
                            FStrPart::Expr(sub_e) => {
                                let (eslot, ety) = self.emit_expr(sub_e, f);
                                self.emit_print_typed_value(eslot, &ety);
                            }
                        }
                    }
                } else {
                    let (aslot, aty) = self.emit_expr(&a.expr, f);
                    self.emit_print_typed_value(aslot, &aty);
                }
            }
            if callee == "println" {
                let (nl_off, _) = self.mc.intern_str("\n");
                self.mc.emit_lea_rax_rdata(nl_off);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]);
                self.mc
                    .code
                    .extend_from_slice(&[0xBA, 0x01, 0x00, 0x00, 0x00]);
                self.mc.emit_call_fn(HELPER_PRINT_BYTES);
            }
            let z = self.alloc_slots(1);
            self.emit_mov_rax_imm64(0);
            self.emit_store_rax_slot(z, 0);
            return (z, Ty::Unit);
        }

        // 2. High-resolution nanosecond hardware timer `clock_ns()` / `time_ns()`
        if callee == "clock_ns" || callee == "time_ns" {
            self.mc.emit_call_fn(HELPER_CLOCK_NS);
            let dst = self.alloc_slots(1);
            self.emit_store_rax_slot(dst, 0);
            return (dst, Ty::Int);
        }

        // 3. `table_fill_f32(table, col_idx, val_f64)` helper for benchmark initialization!
        if callee == "table_fill_f32" && args.len() == 3 {
            let (tslot, _) = self.emit_expr(&args[0].expr, f);
            let (cslot, _) = self.emit_expr(&args[1].expr, f);
            let (vslot, _) = self.emit_expr(&args[2].expr, f);
            // Convert vslot f64 -> f32 in EAX
            self.emit_load_slot_rax(vslot, 0);
            self.mc
                .code
                .extend_from_slice(&[0x66, 0x48, 0x0F, 0x6E, 0xC0]); // MOVQ XMM0, RAX
            self.mc.code.extend_from_slice(&[0xF2, 0x0F, 0x5A, 0xC0]); // CVTSD2SS XMM0, XMM0
            self.mc.code.extend_from_slice(&[0x66, 0x0F, 0x7E, 0xC2]); // MOVD EDX, XMM0 (f32 bits in EDX!)
            // R8 = base_ptr, R9 = len, RCX = col_idx * len * 4
            self.emit_load_slot_rax(tslot, 0);
            self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // MOV R8, RAX
            self.emit_load_slot_rax(tslot, 1);
            self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC1]); // MOV R9, RAX (len)
            self.emit_load_slot_rax(cslot, 0);
            self.mc
                .code
                .extend_from_slice(&[0x49, 0x0F, 0xAF, 0xC1, 0x48, 0xC1, 0xE0, 0x02]); // IMUL RAX, R9; SHL RAX, 2
            self.mc.code.extend_from_slice(&[0x49, 0x01, 0xC0]); // ADD R8, RAX (col start)
            self.mc.code.extend_from_slice(&[0x31, 0xC9]); // XOR ECX, ECX
            let fill_top = self.mc.code.len();
            // MOV [R8 + RCX*4], EDX; INC RCX; CMP RCX, R9; JB fill_top
            self.mc
                .code
                .extend_from_slice(&[0x41, 0x89, 0x14, 0x88, 0x48, 0xFF, 0xC1, 0x4C, 0x39, 0xC9, 0x72]);
            let rel = (fill_top as isize - (self.mc.code.len() as isize + 1)) as i8;
            self.mc.code.push(rel as u8);
            let z = self.alloc_slots(1);
            return (z, Ty::Unit);
        }

        if callee == "table_get_f32" && args.len() == 3 {
            let (tslot, _) = self.emit_expr(&args[0].expr, f);
            let (cslot, _) = self.emit_expr(&args[1].expr, f);
            let (rslot, _) = self.emit_expr(&args[2].expr, f);
            self.emit_load_slot_rax(tslot, 0);
            self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // MOV R8, base_ptr
            self.emit_load_slot_rax(tslot, 1);
            self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC1]); // MOV R9, len
            self.emit_load_slot_rax(rslot, 0);
            self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // MOV RCX, row_idx
            self.mc.code.extend_from_slice(&[0x4C, 0x39, 0xC9]); // CMP RCX, R9
            self.mc.emit_jmp_fn(0x83, HELPER_SEC_TRAP); // JAE trap
            self.emit_load_slot_rax(cslot, 0);
            self.mc
                .code
                .extend_from_slice(&[0x49, 0x0F, 0xAF, 0xC1, 0x48, 0xC1, 0xE0, 0x02]); // IMUL RAX, R9; SHL RAX, 2
            self.mc.code.extend_from_slice(&[0x49, 0x01, 0xC0]); // ADD R8, RAX
            self.mc
                .code
                .extend_from_slice(&[0xF3, 0x41, 0x0F, 0x10, 0x04, 0x88]); // MOVSS XMM0, [R8+RCX*4]
            self.mc.code.extend_from_slice(&[0xF3, 0x0F, 0x5A, 0xC0]); // CVTSS2SD
            self.mc
                .code
                .extend_from_slice(&[0x66, 0x48, 0x0F, 0x7E, 0xC0]); // MOVQ RAX, XMM0
            let dst = self.alloc_slots(1);
            self.emit_store_rax_slot(dst, 0);
            return (dst, Ty::Float);
        }

        // 3b. `@whitebox` GF(2^8) Dual-Bijective S-Box Lookup (`whitebox_sbox(seed, byte_val)`)
        if callee == "whitebox_sbox" && args.len() == 2 {
            let seed_key = if let Expr::Int(k, _) = &args[0].expr {
                *k as u64
            } else {
                let _ = self.emit_expr(&args[0].expr, f);
                0x5A17_C0DE_9988_7766u64
            };
            let tbl_off = self.mc.intern_whitebox_table(seed_key);
            let (vslot, _) = self.emit_expr(&args[1].expr, f);
            self.mc.emit_lea_rax_rdata(tbl_off);
            self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // MOV R8, &table[0]
            self.emit_load_slot_rax(vslot, 0);
            self.mc.code.extend_from_slice(&[0x0F, 0xB6, 0xC8]); // MOVZX ECX, AL
            self.mc
                .code
                .extend_from_slice(&[0x41, 0x0F, 0xB6, 0x04, 0x88]); // MOVZX EAX, BYTE [R8 + RCX*4]
            let dst = self.alloc_slots(1);
            self.emit_store_rax_slot(dst, 0);
            return (dst, Ty::Int);
        }

        // 3c. Native Safe Dynamic Buffers (`ByteBuf`, `IntBuf`), Strings, CLI & File I/O
        if let Some(res) = self.try_emit_buffer_or_io_intrinsic(callee, args, f) {
            return res;
        }

        // 4. `ok(val)` and `err(code)` Zero-Cost Result Constructors
        if callee == "ok" {
            let (vslot, vty) = args
                .first()
                .map(|a| self.emit_expr(&a.expr, f))
                .unwrap_or_else(|| (self.alloc_slots(1), Ty::Int));
            let vcnt = self.mc.type_slots(&vty);
            let dst = self.alloc_slots(1 + vcnt);
            self.emit_mov_rax_imm64(0); // err_tag = 0 (OK!)
            self.emit_store_rax_slot(dst, 0);
            for s in 0..vcnt {
                self.emit_load_slot_rax(vslot, s);
                self.emit_store_rax_slot(dst, 1 + s);
            }
            return (dst, Ty::Result(Box::new(vty), Box::new(Ty::Int)));
        }
        if callee == "err" {
            let (eslot, ety) = args
                .first()
                .map(|a| self.emit_expr(&a.expr, f))
                .unwrap_or_else(|| (self.alloc_slots(1), Ty::Int));
            let rslots = self.meta.ret_slots.max(2);
            let dst = self.alloc_slots(rslots);
            self.emit_load_slot_rax(eslot, 0);
            // Ensure err_tag is non-zero even if user passed `err(0)`
            self.mc
                .code
                .extend_from_slice(&[0x48, 0x85, 0xC0, 0x75, 0x05, 0xB8, 0x01, 0x00, 0x00, 0x00]);
            self.emit_store_rax_slot(dst, 0);
            for s in 1..rslots {
                self.emit_mov_rax_imm64(0);
                self.emit_store_rax_slot(dst, s);
            }
            return (dst, Ty::Result(Box::new(Ty::Any), Box::new(ety)));
        }

        // 5. Struct constructor `StructName(f0, f1, ...)` (supports positional + named fields!)
        if let Some(slayout) = self.mc.checked.structs.get(callee).cloned() {
            let sty = Ty::Struct(callee.to_string());
            let total_slots = self.mc.type_slots(&sty);
            let dst = self.alloc_slots(total_slots);
            let mut cur = 0;
            for (fi, (fname, fty)) in slayout.fields.iter().enumerate() {
                let cnt = self.mc.type_slots(fty);
                let matched = args
                    .iter()
                    .find(|a| a.name.as_deref() == Some(fname.as_str()))
                    .or_else(|| {
                        args.get(fi)
                            .filter(|a| a.name.is_none())
                    });
                if let Some(a) = matched {
                    let (aslot, _) = self.emit_expr(&a.expr, f);
                    for s in 0..cnt {
                        if cur + s < total_slots {
                            self.emit_load_slot_rax(aslot, s);
                            self.emit_store_rax_slot(dst, cur + s);
                        }
                    }
                } else {
                    self.emit_mov_rax_imm64(0);
                    for s in 0..cnt {
                        if cur + s < total_slots {
                            self.emit_store_rax_slot(dst, cur + s);
                        }
                    }
                }
                cur += cnt;
            }
            return (dst, sty);
        }

        // 6. Duck-Typed Interface Static Dispatch or User Function Call
        let mut resolved_callee = callee.to_string();
        if !self.mc.fn_metas.contains_key(&resolved_callee) && !args.is_empty() {
            let (_, first_ty) = self.emit_expr(&args[0].expr, f);
            if let Ty::Struct(sname) = first_ty {
                let candidate = format!("{}__{}", sname, callee);
                if self.mc.fn_metas.contains_key(&candidate) {
                    resolved_callee = candidate;
                }
            }
        }

        let Some(callee_meta) = self.mc.fn_metas.get(&resolved_callee).cloned() else {
            let z = self.alloc_slots(1);
            self.emit_mov_rax_imm64(0);
            self.emit_store_rax_slot(z, 0);
            return (z, Ty::Int);
        };

        // Evaluate arguments (supporting Named Arguments + Default Parameter Values!)
        let callee_decl = &self.mc.prog.funcs[callee_meta.idx];
        let mut evaled_args: Vec<(usize, usize, Option<(usize, usize)>)> = Vec::new();
        for (pi, pdecl) in callee_decl.params.iter().enumerate() {
            let p_slots = self.mc.type_slots(&callee_meta.param_tys[pi]);
            let matched_arg = args
                .iter()
                .find(|ca| ca.name.as_deref() == Some(pdecl.name.as_str()))
                .or_else(|| args.get(pi).filter(|ca| ca.name.is_none()));
            if let Some(ca) = matched_arg {
                let writeback_lval = if ca.mode == ArgPassMode::Mut {
                    self.resolve_lvalue_slot(&ca.expr).map(|(s, o, _)| (s, o))
                } else {
                    None
                };
                let (mut aslot, aty) = self.emit_expr(&ca.expr, f);
                if matches!(callee_meta.param_tys[pi], Ty::Interface(_)) {
                    if let Ty::Struct(sname) = &aty {
                        let s_slots = self.mc.type_slots(&aty);
                        let packed = self.alloc_slots(p_slots);
                        let sid = self.mc.struct_type_id(sname);
                        self.emit_mov_rax_imm64(sid);
                        self.emit_store_rax_slot(packed, 0);
                        for k in 0..s_slots {
                            if 1 + k < p_slots {
                                self.emit_load_slot_rax(aslot, k);
                                self.emit_store_rax_slot(packed, 1 + k);
                            }
                        }
                        aslot = packed;
                    }
                }
                evaled_args.push((aslot, p_slots, writeback_lval));
            } else if let Some(def_expr) = &pdecl.default_val {
                let def_clone = def_expr.clone();
                let (dslot, _) = self.emit_expr(&def_clone, f);
                evaled_args.push((dslot, p_slots, None));
            } else {
                let z = self.alloc_slots(p_slots);
                evaled_args.push((z, p_slots, None));
            }
        }

        let ret_slots = callee_meta.ret_slots;
        let ret_dst = self.alloc_slots(ret_slots);

        if callee_meta.use_fast_abi {
            let reg_loads: [&[u8]; 4] = [
                &[0x48, 0x8B, 0x8D], // MOV RCX, [RBP+disp32]
                &[0x48, 0x8B, 0x95], // MOV RDX, [RBP+disp32]
                &[0x4C, 0x8B, 0x85], // MOV R8, [RBP+disp32]
                &[0x4C, 0x8B, 0x8D], // MOV R9, [RBP+disp32]
            ];
            for (i, (aslot, _, _)) in evaled_args.iter().enumerate() {
                let disp = Self::slot_disp(*aslot, 0);
                self.mc.code.extend_from_slice(reg_loads[i]);
                self.mc.code.extend_from_slice(&disp.to_le_bytes());
            }
            self.mc.emit_call_fn(callee_meta.idx);
            self.emit_store_rax_slot(ret_dst, 0);
            if ret_slots >= 2 {
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xD0]); // MOV RAX, RDX
                self.emit_store_rax_slot(ret_dst, 1);
            }
        } else {
            // Copy-in / Copy-out Value-Result ABI (`inout` Second-Class Reference writeback!)
            let arg_buf_slots = callee_meta.total_arg_slots.max(1);
            let call_arg_buf = self.alloc_slots(arg_buf_slots);
            // Note: contiguous upward in memory requires placing slot 0 at lowest address `[RBP - (call_arg_buf + arg_buf_slots - 1)*8]`!
            let arg_lowest = call_arg_buf + arg_buf_slots - 1;
            let ret_lowest = ret_dst + ret_slots - 1;

            self.emit_lea_rax_slot(arg_lowest, 0);
            self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // MOV R8, RAX (arg_buf base)
            let mut cur_off = 0usize;
            for (aslot, cnt, _) in &evaled_args {
                for s in 0..*cnt {
                    self.emit_load_slot_rax(*aslot, s);
                    let byte_off = ((cur_off + s) * 8) as i32;
                    self.mc.code.extend_from_slice(&[0x49, 0x89, 0x80]); // MOV [R8+byte_off], RAX
                    self.mc.code.extend_from_slice(&byte_off.to_le_bytes());
                }
                cur_off += *cnt;
            }

            // RCX = arg_buf_base, RDX = ret_buf_base
            self.emit_lea_rax_slot(arg_lowest, 0);
            self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // MOV RCX, RAX
            self.emit_lea_rax_slot(ret_lowest, 0);
            self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC2]); // MOV RDX, RAX
            self.mc.emit_call_fn(callee_meta.idx);

            // Unpack upward `ret_lowest` buffer into standard slot order `ret_dst + 0..ret_slots`
            if ret_slots == 1 {
                self.emit_store_rax_slot(ret_dst, 0);
            } else if ret_slots > 1 {
                let tmp = self.alloc_slots(ret_slots);
                self.emit_lea_rax_slot(ret_lowest, 0);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // MOV R8, RAX
                for s in 0..ret_slots {
                    let boff = (s * 8) as i32;
                    self.mc.code.extend_from_slice(&[0x49, 0x8B, 0x80]);
                    self.mc.code.extend_from_slice(&boff.to_le_bytes());
                    self.emit_store_rax_slot(tmp, s);
                }
                for s in 0..ret_slots {
                    self.emit_load_slot_rax(tmp, s);
                    self.emit_store_rax_slot(ret_dst, s);
                }
            }

            // Write back `mut` arguments (`inout` Second-Class Reference semantics!)
            let mut wb_off = 0usize;
            for (_, cnt, wb_opt) in &evaled_args {
                if let Some((lval_base, lval_sub)) = wb_opt {
                    self.emit_lea_rax_slot(arg_lowest, 0);
                    self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // MOV R8, RAX
                    for s in 0..*cnt {
                        let boff = ((wb_off + s) * 8) as i32;
                        self.mc.code.extend_from_slice(&[0x49, 0x8B, 0x80]);
                        self.mc.code.extend_from_slice(&boff.to_le_bytes());
                        self.emit_store_rax_slot(*lval_base, *lval_sub + s);
                    }
                }
                wb_off += *cnt;
            }
        }

        (ret_dst, callee_meta.ret_ty)
    }

    fn emit_print_typed_value(&mut self, slot: usize, ty: &Ty) {
        match ty {
            Ty::Str => {
                self.emit_load_slot_rax(slot, 1);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC2]); // MOV RDX, RAX (len)
                self.emit_load_slot_rax(slot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // MOV RCX, RAX (ptr)
                self.mc.emit_call_fn(HELPER_PRINT_BYTES);
            }
            Ty::Dec => {
                self.emit_load_slot_rax(slot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // MOV RCX, RAX
                self.mc.emit_call_fn(HELPER_PRINT_DEC);
            }
            Ty::Float => {
                // Convert f64 -> scaled 10_000 fixed-point and print via `print_dec`!
                self.emit_load_slot_rax(slot, 0);
                self.mc
                    .code
                    .extend_from_slice(&[0x66, 0x48, 0x0F, 0x6E, 0xC0]); // MOVQ XMM0, RAX
                self.mc
                    .code
                    .extend_from_slice(&[0xB8, 0x10, 0x27, 0x00, 0x00]); // 10000
                self.mc
                    .code
                    .extend_from_slice(&[0xF2, 0x48, 0x0F, 0x2A, 0xC8]); // CVTSI2SD XMM1, RAX
                self.mc.code.extend_from_slice(&[0xF2, 0x0F, 0x59, 0xC1]); // MULSD XMM0, XMM1
                self.mc
                    .code
                    .extend_from_slice(&[0xF2, 0x48, 0x0F, 0x2C, 0xC8]); // CVTTSD2SI RCX, XMM0
                self.mc.emit_call_fn(HELPER_PRINT_DEC);
            }
            Ty::Bool => {
                let (t_off, _) = self.mc.intern_str("true");
                let (f_off, _) = self.mc.intern_str("false");
                self.emit_load_slot_rax(slot, 0);
                self.mc
                    .code
                    .extend_from_slice(&[0x48, 0x85, 0xC0, 0x74, 0x0E]); // TEST RAX, RAX; JZ .false
                self.mc.emit_lea_rax_rdata(t_off);
                self.mc
                    .code
                    .extend_from_slice(&[0xBA, 0x04, 0x00, 0x00, 0x00, 0xEB, 0x0C]);
                self.mc.emit_lea_rax_rdata(f_off);
                self.mc
                    .code
                    .extend_from_slice(&[0xBA, 0x05, 0x00, 0x00, 0x00]);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]);
                self.mc.emit_call_fn(HELPER_PRINT_BYTES);
            }
            _ => {
                self.emit_load_slot_rax(slot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // MOV RCX, RAX
                self.mc.emit_call_fn(HELPER_PRINT_I64);
            }
        }
    }

    fn emit_scalar_binop(
        &mut self,
        op: BinOp,
        lslot: usize,
        loff: usize,
        rslot: usize,
        roff: usize,
        ty: &Ty,
    ) -> usize {
        let dst = self.alloc_slots(1);
        self.emit_load_slot_rax(lslot, loff);
        let dr = Self::slot_disp(rslot, roff);
        self.mc.code.extend_from_slice(&[0x48, 0x8B, 0x8D]); // MOV RCX, [RBP + dr]
        self.mc.code.extend_from_slice(&dr.to_le_bytes());

        if *ty == Ty::Float {
            self.mc
                .code
                .extend_from_slice(&[0x66, 0x48, 0x0F, 0x6E, 0xC0]); // MOVQ XMM0, RAX
            self.mc
                .code
                .extend_from_slice(&[0x66, 0x48, 0x0F, 0x6E, 0xC9]); // MOVQ XMM1, RCX
            match op {
                BinOp::Add | BinOp::WrapAdd => self.mc.code.extend_from_slice(&[0xF2, 0x0F, 0x58, 0xC1]),
                BinOp::Sub | BinOp::WrapSub => self.mc.code.extend_from_slice(&[0xF2, 0x0F, 0x5C, 0xC1]),
                BinOp::Mul | BinOp::WrapMul => self.mc.code.extend_from_slice(&[0xF2, 0x0F, 0x59, 0xC1]),
                BinOp::Div => self.mc.code.extend_from_slice(&[0xF2, 0x0F, 0x5E, 0xC1]),
                _ => {
                    // Float comparisons via UCOMISD XMM0, XMM1
                    self.mc.code.extend_from_slice(&[0x66, 0x0F, 0x2E, 0xC1]);
                    let setcc = match op {
                        BinOp::Eq => 0x94,    // SETE
                        BinOp::NotEq => 0x95, // SETNE
                        BinOp::Lt => 0x92,    // SETB
                        BinOp::LtEq => 0x96,  // SETBE
                        BinOp::Gt => 0x97,    // SETA
                        BinOp::GtEq => 0x93,  // SETAE
                        _ => 0x94,
                    };
                    self.mc
                        .code
                        .extend_from_slice(&[0x0F, setcc, 0xC0, 0x0F, 0xB6, 0xC0]);
                    self.emit_store_rax_slot(dst, 0);
                    return dst;
                }
            }
            self.mc
                .code
                .extend_from_slice(&[0x66, 0x48, 0x0F, 0x7E, 0xC0]); // MOVQ RAX, XMM0
            self.emit_store_rax_slot(dst, 0);
            return dst;
        }

        // Fixed-Point Decimal `dec * dec` and `dec / dec` scaling (scale = 10_000)
        if *ty == Ty::Dec && (op == BinOp::Mul || op == BinOp::Div) {
            if op == BinOp::Mul {
                self.mc.code.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC1]); // IMUL RAX, RCX
                self.mc
                    .code
                    .extend_from_slice(&[0x48, 0xC7, 0xC1, 0x10, 0x27, 0x00, 0x00]); // 10000
                self.mc
                    .code
                    .extend_from_slice(&[0x48, 0x99, 0x48, 0xF7, 0xF9]); // CQO; IDIV RCX
            } else {
                self.mc.code.extend_from_slice(&[0x48, 0x85, 0xC9]); // TEST RCX, RCX
                self.mc.emit_jmp_fn(0x84, HELPER_SEC_TRAP); // JZ trap
                self.mc
                    .code
                    .extend_from_slice(&[0x48, 0x69, 0xC0, 0x10, 0x27, 0x00, 0x00]); // IMUL RAX, 10000
                self.mc
                    .code
                    .extend_from_slice(&[0x48, 0x99, 0x48, 0xF7, 0xF9]); // CQO; IDIV RCX
            }
            self.emit_store_rax_slot(dst, 0);
            return dst;
        }

        match op {
            BinOp::Add => {
                self.mc.code.extend_from_slice(&[0x48, 0x01, 0xC8]); // ADD RAX, RCX
                if self.mc.shield_enabled {
                    self.mc.emit_jmp_fn(0x80, HELPER_SEC_TRAP); // JO hardware overflow trap!
                }
            }
            BinOp::WrapAdd => {
                self.mc.code.extend_from_slice(&[0x48, 0x01, 0xC8]);
            }
            BinOp::Sub => {
                self.mc.code.extend_from_slice(&[0x48, 0x29, 0xC8]); // SUB RAX, RCX
                if self.mc.shield_enabled {
                    self.mc.emit_jmp_fn(0x80, HELPER_SEC_TRAP);
                }
            }
            BinOp::WrapSub => {
                self.mc.code.extend_from_slice(&[0x48, 0x29, 0xC8]);
            }
            BinOp::Mul => {
                self.mc.code.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC1]); // IMUL RAX, RCX
                if self.mc.shield_enabled {
                    self.mc.emit_jmp_fn(0x80, HELPER_SEC_TRAP);
                }
            }
            BinOp::WrapMul => {
                self.mc.code.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC1]);
            }
            BinOp::Div => {
                // Hardware Division-by-Zero & INT64_MIN/-1 Guard!
                self.mc.code.extend_from_slice(&[0x48, 0x85, 0xC9]); // TEST RCX, RCX
                self.mc.emit_jmp_fn(0x84, HELPER_SEC_TRAP); // JZ trap
                // Check RCX == -1 && RAX == i64::MIN
                self.mc.code.extend_from_slice(&[0x48, 0x83, 0xF9, 0xFF, 0x75, 0x10]); // CMP RCX, -1; JNE .ok
                self.mc.code.extend_from_slice(&[0x48, 0xBA]); // MOV RDX, i64::MIN
                self.mc.code.extend_from_slice(&i64::MIN.to_le_bytes());
                self.mc.code.extend_from_slice(&[0x48, 0x39, 0xD0]); // CMP RAX, RDX
                self.mc.emit_jmp_fn(0x84, HELPER_SEC_TRAP); // JE trap
                // .ok: Note JNE rel8 skips 10 (MOV RDX) + 3 (CMP) + 6 (JE rel32) = 19 (0x13) bytes!
                let fix_jne = self.mc.code.len() - 20;
                self.mc.code[fix_jne] = 0x13;
                self.mc
                    .code
                    .extend_from_slice(&[0x48, 0x99, 0x48, 0xF7, 0xF9]); // CQO; IDIV RCX
            }
            BinOp::Mod => {
                self.mc.code.extend_from_slice(&[0x48, 0x85, 0xC9]);
                self.mc.emit_jmp_fn(0x84, HELPER_SEC_TRAP);
                self.mc
                    .code
                    .extend_from_slice(&[0x48, 0x99, 0x48, 0xF7, 0xF9, 0x48, 0x89, 0xD0]); // CQO; IDIV RCX; MOV RAX, RDX
            }
            BinOp::BitAnd => self.mc.code.extend_from_slice(&[0x48, 0x21, 0xC8]),
            BinOp::BitOr => self.mc.code.extend_from_slice(&[0x48, 0x09, 0xC8]),
            BinOp::BitXor => self.mc.code.extend_from_slice(&[0x48, 0x31, 0xC8]),
            BinOp::Shl => self.mc.code.extend_from_slice(&[0x48, 0xD3, 0xE0]), // SHL RAX, CL
            BinOp::Shr => self.mc.code.extend_from_slice(&[0x48, 0xD3, 0xF8]), // SAR RAX, CL
            BinOp::And => {
                // (RAX != 0) & (RCX != 0)
                self.mc.code.extend_from_slice(&[
                    0x48, 0x85, 0xC0, 0x0F, 0x95, 0xC0, 0x48, 0x85, 0xC9, 0x0F, 0x95, 0xC1, 0x20,
                    0xC8, 0x0F, 0xB6, 0xC0,
                ]);
            }
            BinOp::Or => {
                self.mc.code.extend_from_slice(&[
                    0x48, 0x09, 0xC8, 0x0F, 0x95, 0xC0, 0x0F, 0xB6, 0xC0,
                ]);
            }
            BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => {
                self.mc.code.extend_from_slice(&[0x48, 0x39, 0xC8]); // CMP RAX, RCX
                let setcc = match op {
                    BinOp::Eq => 0x94,    // SETE
                    BinOp::NotEq => 0x95, // SETNE
                    BinOp::Lt => 0x9C,    // SETL
                    BinOp::LtEq => 0x9E,  // SETLE
                    BinOp::Gt => 0x9F,    // SETG
                    BinOp::GtEq => 0x9D,  // SETGE
                    _ => unreachable!(),
                };
                self.mc
                    .code
                    .extend_from_slice(&[0x0F, setcc, 0xC0, 0x0F, 0xB6, 0xC0]);
            }
        }
        self.emit_store_rax_slot(dst, 0);
        dst
    }

    fn try_emit_buffer_or_io_intrinsic(
        &mut self,
        callee: &str,
        args: &[CallArg],
        f: &FnDecl,
    ) -> Option<(usize, Ty)> {
        match callee {
            "buf_new" if args.len() == 1 => {
                let (cslot, _) = self.emit_expr(&args[0].expr, f);
                self.emit_load_slot_rax(cslot, 0);
                // Minimum 65536-byte capacity for zero-realloc header-prefixed ByteBuf
                self.mc.code.extend_from_slice(&[
                    0x48, 0x3D, 0x00, 0x00, 0x01, 0x00, 0x7D, 0x05, 0xB8, 0x00, 0x00, 0x01, 0x00,
                ]);
                self.emit_store_rax_slot(cslot, 0);
                self.mc
                    .code
                    .extend_from_slice(&[0x48, 0x89, 0xC2, 0x48, 0x83, 0xC2, 0x40]); // RDX = cap + 64
                self.mc.code.extend_from_slice(&[0x31, 0xC9]); // RCX = 0
                self.mc
                    .code
                    .extend_from_slice(&[0x41, 0xB8, 0x00, 0x30, 0x00, 0x00]); // 0x3000
                self.mc
                    .code
                    .extend_from_slice(&[0x41, 0xB9, 0x04, 0x00, 0x00, 0x00]); // PAGE_READWRITE
                self.mc.emit_call_iat(IAT_VIRTUAL_ALLOC);
                let dc = Self::slot_disp(cslot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x8B, 0x8D]); // MOV RCX, [cslot]
                self.mc.code.extend_from_slice(&dc.to_le_bytes());
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0x48, 0x08]); // [RAX + 8] = cap
                let dst = self.alloc_slots(1);
                self.emit_store_rax_slot(dst, 0);
                Some((dst, Ty::Struct("ByteBuf".into())))
            }
            "ibuf_new" if args.len() == 1 => {
                let (cslot, _) = self.emit_expr(&args[0].expr, f);
                self.emit_load_slot_rax(cslot, 0);
                // Minimum 8192 elements
                self.mc.code.extend_from_slice(&[
                    0x48, 0x3D, 0x00, 0x20, 0x00, 0x00, 0x7D, 0x05, 0xB8, 0x00, 0x20, 0x00, 0x00,
                ]);
                self.emit_store_rax_slot(cslot, 0);
                self.mc.code.extend_from_slice(&[
                    0x48, 0x89, 0xC2, 0x48, 0xC1, 0xE2, 0x03, 0x48, 0x83, 0xC2, 0x40,
                ]); // RDX = cap*8 + 64
                self.mc.code.extend_from_slice(&[0x31, 0xC9]);
                self.mc
                    .code
                    .extend_from_slice(&[0x41, 0xB8, 0x00, 0x30, 0x00, 0x00]);
                self.mc
                    .code
                    .extend_from_slice(&[0x41, 0xB9, 0x04, 0x00, 0x00, 0x00]);
                self.mc.emit_call_iat(IAT_VIRTUAL_ALLOC);
                let dc = Self::slot_disp(cslot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x8B, 0x8D]);
                self.mc.code.extend_from_slice(&dc.to_le_bytes());
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0x48, 0x08]); // [RAX + 8] = cap
                let dst = self.alloc_slots(1);
                self.emit_store_rax_slot(dst, 0);
                Some((dst, Ty::Struct("IntBuf".into())))
            }
            "buf_len" | "ibuf_len" if args.len() == 1 => {
                let (bslot, _) = self.emit_expr(&args[0].expr, f);
                self.emit_load_slot_rax(bslot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x8B, 0x00]); // MOV RAX, [RAX]
                let dst = self.alloc_slots(1);
                self.emit_store_rax_slot(dst, 0);
                Some((dst, Ty::Int))
            }
            "buf_get" if args.len() == 2 => {
                let (bslot, _) = self.emit_expr(&args[0].expr, f);
                let (islot, _) = self.emit_expr(&args[1].expr, f);
                self.emit_load_slot_rax(bslot, 0);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // MOV R8, buf
                self.emit_load_slot_rax(islot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // MOV RCX, idx
                self.mc.code.extend_from_slice(&[0x49, 0x3B, 0x08]); // CMP RCX, [R8]
                self.mc.emit_jmp_fn(0x83, HELPER_SEC_TRAP); // JAE trap
                self.mc
                    .code
                    .extend_from_slice(&[0x41, 0x0F, 0xB6, 0x44, 0x08, 0x10]); // MOVZX EAX, BYTE [R8+RCX+16]
                let dst = self.alloc_slots(1);
                self.emit_store_rax_slot(dst, 0);
                Some((dst, Ty::Int))
            }
            "buf_set" if args.len() == 3 => {
                let (bslot, _) = self.emit_expr(&args[0].expr, f);
                let (islot, _) = self.emit_expr(&args[1].expr, f);
                let (vslot, _) = self.emit_expr(&args[2].expr, f);
                self.emit_load_slot_rax(bslot, 0);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // MOV R8, buf
                self.emit_load_slot_rax(islot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // MOV RCX, idx
                self.mc.code.extend_from_slice(&[0x49, 0x3B, 0x08]); // CMP RCX, [R8]
                self.mc.emit_jmp_fn(0x83, HELPER_SEC_TRAP);
                self.emit_load_slot_rax(vslot, 0);
                self.mc
                    .code
                    .extend_from_slice(&[0x41, 0x88, 0x44, 0x08, 0x10]); // MOV BYTE [R8+RCX+16], AL
                let z = self.alloc_slots(1);
                Some((z, Ty::Unit))
            }
            "buf_put_u32" | "buf_put_u64" if args.len() == 3 => {
                let width: u8 = if callee == "buf_put_u32" { 4 } else { 8 };
                let (bslot, _) = self.emit_expr(&args[0].expr, f);
                let (islot, _) = self.emit_expr(&args[1].expr, f);
                let (vslot, _) = self.emit_expr(&args[2].expr, f);
                self.emit_load_slot_rax(bslot, 0);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // MOV R8, buf
                self.emit_load_slot_rax(islot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // MOV RCX, idx
                self.mc.code.extend_from_slice(&[0x48, 0x85, 0xC9]); // TEST RCX, RCX
                self.mc.emit_jmp_fn(0x88, HELPER_SEC_TRAP); // JS trap
                self.mc.code.extend_from_slice(&[0x48, 0x8D, 0x51, width]); // LEA RDX, [RCX + width]
                self.mc.code.extend_from_slice(&[0x49, 0x3B, 0x10]); // CMP RDX, [R8]
                self.mc.emit_jmp_fn(0x87, HELPER_SEC_TRAP); // JA trap
                self.emit_load_slot_rax(vslot, 0);
                if width == 4 {
                    self.mc
                        .code
                        .extend_from_slice(&[0x41, 0x89, 0x44, 0x08, 0x10]); // MOV DWORD [R8+RCX+16], EAX
                } else {
                    self.mc
                        .code
                        .extend_from_slice(&[0x49, 0x89, 0x44, 0x08, 0x10]); // MOV QWORD [R8+RCX+16], RAX
                }
                let z = self.alloc_slots(1);
                Some((z, Ty::Unit))
            }
            "buf_push" | "buf_push_u16" | "buf_push_u32" | "buf_push_u64" if args.len() == 2 => {
                let width: u8 = match callee {
                    "buf_push" => 1,
                    "buf_push_u16" => 2,
                    "buf_push_u32" => 4,
                    _ => 8,
                };
                let (bslot, _) = self.emit_expr(&args[0].expr, f);
                let (vslot, _) = self.emit_expr(&args[1].expr, f);
                self.emit_load_slot_rax(bslot, 0);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // MOV R8, buf
                self.mc.code.extend_from_slice(&[0x49, 0x8B, 0x08]); // MOV RCX, [R8] (len)
                self.mc.code.extend_from_slice(&[0x48, 0x8D, 0x51, width]); // LEA RDX, [RCX + width]
                self.mc.code.extend_from_slice(&[0x49, 0x3B, 0x50, 0x08]); // CMP RDX, [R8 + 8] (cap)
                self.mc.emit_jmp_fn(0x87, HELPER_SEC_TRAP); // JA trap
                self.emit_load_slot_rax(vslot, 0);
                match width {
                    1 => self
                        .mc
                        .code
                        .extend_from_slice(&[0x41, 0x88, 0x44, 0x08, 0x10]),
                    2 => self
                        .mc
                        .code
                        .extend_from_slice(&[0x66, 0x41, 0x89, 0x44, 0x08, 0x10]),
                    4 => self
                        .mc
                        .code
                        .extend_from_slice(&[0x41, 0x89, 0x44, 0x08, 0x10]),
                    _ => self
                        .mc
                        .code
                        .extend_from_slice(&[0x49, 0x89, 0x44, 0x08, 0x10]),
                }
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0x10]); // MOV [R8], RDX (len += width)
                let z = self.alloc_slots(1);
                Some((z, Ty::Unit))
            }
            "buf_push_str" if args.len() == 2 => {
                let (bslot, _) = self.emit_expr(&args[0].expr, f);
                let (sslot, _) = self.emit_expr(&args[1].expr, f);
                self.emit_load_slot_rax(bslot, 0);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // R8 = buf
                self.emit_load_slot_rax(sslot, 0);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC1]); // R9 = str_ptr
                self.emit_load_slot_rax(sslot, 1);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC2]); // R10 = str_len
                self.mc.code.extend_from_slice(&[0x49, 0x8B, 0x08]); // RCX = [R8] (old_len)
                self.mc
                    .code
                    .extend_from_slice(&[0x48, 0x89, 0xCA, 0x4C, 0x01, 0xD2]); // RDX = old_len + str_len
                self.mc.code.extend_from_slice(&[0x49, 0x3B, 0x50, 0x08]); // CMP RDX, [R8 + 8]
                self.mc.emit_jmp_fn(0x87, HELPER_SEC_TRAP);
                self.mc.code.extend_from_slice(&[0x4C, 0x01, 0xC1]); // ADD RCX, R8 (dst_base - 16)
                self.mc.code.extend_from_slice(&[0x31, 0xC0]); // XOR EAX, EAX
                let cp_top = self.mc.code.len();
                self.mc.code.extend_from_slice(&[0x4C, 0x39, 0xD0, 0x73]); // CMP RAX, R10; JAE .done
                let jae_pos = self.mc.code.len();
                self.mc.code.push(0);
                self.mc
                    .code
                    .extend_from_slice(&[0x45, 0x8A, 0x1C, 0x01]); // MOV R11B, [R9 + RAX]
                self.mc
                    .code
                    .extend_from_slice(&[0x44, 0x88, 0x5C, 0x01, 0x10]); // MOV [RCX + RAX + 16], R11B
                self.mc.code.extend_from_slice(&[0x48, 0xFF, 0xC0, 0xEB]); // INC RAX; JMP cp_top
                let rel_back = (cp_top as isize - (self.mc.code.len() as isize + 1)) as i8;
                self.mc.code.push(rel_back as u8);
                let done_pos = self.mc.code.len();
                self.mc.code[jae_pos] = (done_pos - (jae_pos + 1)) as u8;
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0x10]); // MOV [R8], RDX
                let z = self.alloc_slots(1);
                Some((z, Ty::Unit))
            }
            "buf_slice_str" if args.len() == 3 => {
                let (bslot, _) = self.emit_expr(&args[0].expr, f);
                let (sslot, _) = self.emit_expr(&args[1].expr, f);
                let (eslot, _) = self.emit_expr(&args[2].expr, f);
                self.emit_load_slot_rax(bslot, 0);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // R8 = buf
                self.emit_load_slot_rax(sslot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // RCX = start
                self.emit_load_slot_rax(eslot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC2]); // RDX = end
                self.mc.code.extend_from_slice(&[0x48, 0x39, 0xD1]); // CMP RCX, RDX
                self.mc.emit_jmp_fn(0x87, HELPER_SEC_TRAP);
                self.mc.code.extend_from_slice(&[0x49, 0x3B, 0x10]); // CMP RDX, [R8]
                self.mc.emit_jmp_fn(0x87, HELPER_SEC_TRAP);
                self.mc.code.extend_from_slice(&[0x48, 0x29, 0xCA]); // SUB RDX, RCX
                self.mc
                    .code
                    .extend_from_slice(&[0x49, 0x8D, 0x44, 0x08, 0x10]); // LEA RAX, [R8+RCX+16]
                let dst = self.alloc_slots(2);
                self.emit_store_rax_slot(dst, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xD0]); // MOV RAX, RDX
                self.emit_store_rax_slot(dst, 1);
                Some((dst, Ty::Str))
            }
            "buf_free" if args.len() == 1 => {
                let (bslot, _) = self.emit_expr(&args[0].expr, f);
                self.emit_load_slot_rax(bslot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x85, 0xC0, 0x74]);
                let jz_pos = self.mc.code.len();
                self.mc.code.push(0);
                self.mc
                    .code
                    .extend_from_slice(&[0x48, 0x89, 0xC1, 0x31, 0xD2]);
                self.mc
                    .code
                    .extend_from_slice(&[0x41, 0xB8, 0x00, 0x80, 0x00, 0x00]);
                self.mc.emit_call_iat(IAT_VIRTUAL_FREE);
                let after = self.mc.code.len();
                self.mc.code[jz_pos] = (after - (jz_pos + 1)) as u8;
                if let Expr::Ident(vn, _) | Expr::MoveIdent(vn, _) = &args[0].expr {
                    if let Some(lv) = self.vars.get(vn).cloned() {
                        self.emit_mov_rax_imm64(0);
                        self.emit_store_rax_slot(lv.slot, 0);
                    }
                }
                let z = self.alloc_slots(1);
                Some((z, Ty::Unit))
            }
            "ibuf_push" if args.len() == 2 => {
                let (bslot, _) = self.emit_expr(&args[0].expr, f);
                let (vslot, _) = self.emit_expr(&args[1].expr, f);
                self.emit_load_slot_rax(bslot, 0);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // R8 = ibuf
                self.mc.code.extend_from_slice(&[0x49, 0x8B, 0x08]); // RCX = [R8] (len)
                self.mc.code.extend_from_slice(&[0x49, 0x3B, 0x48, 0x08]); // CMP RCX, [R8 + 8]
                self.mc.emit_jmp_fn(0x83, HELPER_SEC_TRAP); // JAE trap
                self.emit_load_slot_rax(vslot, 0);
                self.mc
                    .code
                    .extend_from_slice(&[0x49, 0x89, 0x44, 0xC8, 0x10]); // MOV [R8 + RCX*8 + 16], RAX
                self.mc.code.extend_from_slice(&[0x49, 0xFF, 0x00]); // INC QWORD [R8]
                let z = self.alloc_slots(1);
                Some((z, Ty::Unit))
            }
            "ibuf_get" if args.len() == 2 => {
                let (bslot, _) = self.emit_expr(&args[0].expr, f);
                let (islot, _) = self.emit_expr(&args[1].expr, f);
                self.emit_load_slot_rax(bslot, 0);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // R8 = ibuf
                self.emit_load_slot_rax(islot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // RCX = idx
                self.mc.code.extend_from_slice(&[0x49, 0x3B, 0x08]); // CMP RCX, [R8]
                self.mc.emit_jmp_fn(0x83, HELPER_SEC_TRAP);
                self.mc
                    .code
                    .extend_from_slice(&[0x49, 0x8B, 0x44, 0xC8, 0x10]); // MOV RAX, [R8 + RCX*8 + 16]
                let dst = self.alloc_slots(1);
                self.emit_store_rax_slot(dst, 0);
                Some((dst, Ty::Int))
            }
            "ibuf_set" if args.len() == 3 => {
                let (bslot, _) = self.emit_expr(&args[0].expr, f);
                let (islot, _) = self.emit_expr(&args[1].expr, f);
                let (vslot, _) = self.emit_expr(&args[2].expr, f);
                self.emit_load_slot_rax(bslot, 0);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // R8 = ibuf
                self.emit_load_slot_rax(islot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // RCX = idx
                self.mc.code.extend_from_slice(&[0x49, 0x3B, 0x08]); // CMP RCX, [R8]
                self.mc.emit_jmp_fn(0x83, HELPER_SEC_TRAP);
                self.emit_load_slot_rax(vslot, 0);
                self.mc
                    .code
                    .extend_from_slice(&[0x49, 0x89, 0x44, 0xC8, 0x10]); // MOV [R8 + RCX*8 + 16], RAX
                let z = self.alloc_slots(1);
                Some((z, Ty::Unit))
            }
            "str_len" if args.len() == 1 => {
                let (sslot, _) = self.emit_expr(&args[0].expr, f);
                let dst = self.alloc_slots(1);
                self.emit_load_slot_rax(sslot, 1);
                self.emit_store_rax_slot(dst, 0);
                Some((dst, Ty::Int))
            }
            "str_byte" if args.len() == 2 => {
                let (sslot, _) = self.emit_expr(&args[0].expr, f);
                let (islot, _) = self.emit_expr(&args[1].expr, f);
                self.emit_load_slot_rax(sslot, 0);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // R8 = ptr
                self.emit_load_slot_rax(sslot, 1);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC1]); // R9 = len
                self.emit_load_slot_rax(islot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // RCX = idx
                self.mc.code.extend_from_slice(&[0x4C, 0x39, 0xC9]); // CMP RCX, R9
                self.mc.emit_jmp_fn(0x83, HELPER_SEC_TRAP);
                self.mc
                    .code
                    .extend_from_slice(&[0x41, 0x0F, 0xB6, 0x04, 0x08]); // MOVZX EAX, BYTE [R8 + RCX]
                let dst = self.alloc_slots(1);
                self.emit_store_rax_slot(dst, 0);
                Some((dst, Ty::Int))
            }
            "str_slice" if args.len() == 3 => {
                let (sslot, _) = self.emit_expr(&args[0].expr, f);
                let (st_slot, _) = self.emit_expr(&args[1].expr, f);
                let (en_slot, _) = self.emit_expr(&args[2].expr, f);
                self.emit_load_slot_rax(sslot, 0);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC0]); // R8 = ptr
                self.emit_load_slot_rax(sslot, 1);
                self.mc.code.extend_from_slice(&[0x49, 0x89, 0xC1]); // R9 = len
                self.emit_load_slot_rax(st_slot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // RCX = start
                self.emit_load_slot_rax(en_slot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC2]); // RDX = end
                self.mc.code.extend_from_slice(&[0x48, 0x39, 0xD1]); // CMP RCX, RDX
                self.mc.emit_jmp_fn(0x87, HELPER_SEC_TRAP);
                self.mc.code.extend_from_slice(&[0x4C, 0x39, 0xCA]); // CMP RDX, R9
                self.mc.emit_jmp_fn(0x87, HELPER_SEC_TRAP);
                self.mc.code.extend_from_slice(&[0x48, 0x29, 0xCA]); // SUB RDX, RCX
                self.mc.code.extend_from_slice(&[0x49, 0x8D, 0x04, 0x08]); // LEA RAX, [R8 + RCX]
                let dst = self.alloc_slots(2);
                self.emit_store_rax_slot(dst, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xD0]); // MOV RAX, RDX
                self.emit_store_rax_slot(dst, 1);
                Some((dst, Ty::Str))
            }
            "str_eq" if args.len() == 2 => {
                let (aslot, _) = self.emit_expr(&args[0].expr, f);
                let (bslot, _) = self.emit_expr(&args[1].expr, f);
                let da0 = Self::slot_disp(aslot, 0);
                let da1 = Self::slot_disp(aslot, 1);
                let db0 = Self::slot_disp(bslot, 0);
                let db1 = Self::slot_disp(bslot, 1);
                self.mc.code.extend_from_slice(&[0x48, 0x8B, 0x8D]);
                self.mc.code.extend_from_slice(&da0.to_le_bytes());
                self.mc.code.extend_from_slice(&[0x48, 0x8B, 0x95]);
                self.mc.code.extend_from_slice(&da1.to_le_bytes());
                self.mc.code.extend_from_slice(&[0x4C, 0x8B, 0x85]);
                self.mc.code.extend_from_slice(&db0.to_le_bytes());
                self.mc.code.extend_from_slice(&[0x4C, 0x8B, 0x8D]);
                self.mc.code.extend_from_slice(&db1.to_le_bytes());
                self.mc.emit_call_fn(HELPER_STR_EQ);
                let dst = self.alloc_slots(1);
                self.emit_store_rax_slot(dst, 0);
                Some((dst, Ty::Bool))
            }
            "args_count" => {
                self.mc.emit_call_fn(HELPER_ARGS_COUNT);
                let dst = self.alloc_slots(1);
                self.emit_store_rax_slot(dst, 0);
                Some((dst, Ty::Int))
            }
            "args_get" if args.len() == 1 => {
                let (islot, _) = self.emit_expr(&args[0].expr, f);
                self.emit_load_slot_rax(islot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xC1]); // MOV RCX, RAX
                self.mc.emit_call_fn(HELPER_ARGS_GET);
                let dst = self.alloc_slots(2);
                self.emit_store_rax_slot(dst, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xD0]); // MOV RAX, RDX
                self.emit_store_rax_slot(dst, 1);
                Some((dst, Ty::Str))
            }
            "fs_read_file" if args.len() == 1 => {
                let (pslot, _) = self.emit_expr(&args[0].expr, f);
                let dp0 = Self::slot_disp(pslot, 0);
                let dp1 = Self::slot_disp(pslot, 1);
                self.mc.code.extend_from_slice(&[0x48, 0x8B, 0x8D]);
                self.mc.code.extend_from_slice(&dp0.to_le_bytes());
                self.mc.code.extend_from_slice(&[0x48, 0x8B, 0x95]);
                self.mc.code.extend_from_slice(&dp1.to_le_bytes());
                self.mc.emit_call_fn(HELPER_FS_READ);
                let dst = self.alloc_slots(2);
                self.emit_store_rax_slot(dst, 0); // err_tag (0=OK, 1=Err)
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xD0]); // MOV RAX, RDX (ByteBuf ptr)
                self.emit_store_rax_slot(dst, 1);
                Some((
                    dst,
                    Ty::Result(Box::new(Ty::Struct("ByteBuf".into())), Box::new(Ty::Int)),
                ))
            }
            "fs_write_file" if args.len() == 2 => {
                let (pslot, _) = self.emit_expr(&args[0].expr, f);
                let (bslot, _) = self.emit_expr(&args[1].expr, f);
                let dp0 = Self::slot_disp(pslot, 0);
                let dp1 = Self::slot_disp(pslot, 1);
                let db0 = Self::slot_disp(bslot, 0);
                self.mc.code.extend_from_slice(&[0x48, 0x8B, 0x8D]);
                self.mc.code.extend_from_slice(&dp0.to_le_bytes());
                self.mc.code.extend_from_slice(&[0x48, 0x8B, 0x95]);
                self.mc.code.extend_from_slice(&dp1.to_le_bytes());
                self.mc.code.extend_from_slice(&[0x4C, 0x8B, 0x85]);
                self.mc.code.extend_from_slice(&db0.to_le_bytes());
                self.mc.emit_call_fn(HELPER_FS_WRITE);
                let dst = self.alloc_slots(2);
                self.emit_store_rax_slot(dst, 0); // err_tag
                self.mc.code.extend_from_slice(&[0x48, 0x89, 0xD0]); // written
                self.emit_store_rax_slot(dst, 1);
                Some((dst, Ty::Result(Box::new(Ty::Int), Box::new(Ty::Int))))
            }
            _ => None,
        }
    }
}
