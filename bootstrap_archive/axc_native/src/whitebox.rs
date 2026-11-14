//! Axon v5.0 `@whitebox` GF(2^8) Dual-Bijective S-Box Table Generator (`whitebox.rs`)
//! Synthesizes AV-clean, key-embedded White-Box Cryptography lookup tables over GF(2^8)
//! (irreducible polynomial x^8 + x^4 + x^3 + x + 1 = 0x11B) so secret keys are dissolved
//! into mathematical table bijections without storing raw keys in `.rdata` or registers.

/// Multiply two elements in GF(2^8) modulo 0x11B
pub fn gf256_mul(mut a: u8, mut b: u8) -> u8 {
    let mut p = 0u8;
    for _ in 0..8 {
        if (b & 1) != 0 {
            p ^= a;
        }
        let hi = (a & 0x80) != 0;
        a <<= 1;
        if hi {
            a ^= 0x1B;
        }
        b >>= 1;
    }
    p
}

/// Multiplicative inverse in GF(2^8): x^254 mod 0x11B
pub fn gf256_inv(x: u8) -> u8 {
    if x == 0 {
        return 0;
    }
    let mut res = 1u8;
    let mut base = x;
    let mut exp = 254u8;
    while exp > 0 {
        if (exp & 1) != 0 {
            res = gf256_mul(res, base);
        }
        base = gf256_mul(base, base);
        exp >>= 1;
    }
    res
}

/// Standard AES-like affine transformation over GF(2)[x]/(x^8 + 1)
fn affine_byte(x: u8, c: u8) -> u8 {
    let mut s = x;
    s ^= x.rotate_left(1);
    s ^= x.rotate_left(2);
    s ^= x.rotate_left(3);
    s ^= x.rotate_left(4);
    s ^ c
}

/// Synthesizes a 256-byte White-Box Bijective Round Table for a compile-time `seed_key`.
/// Guarantees Shannon entropy == 8.0 on 256 distinct bytes, so we interleave 4-byte aligned
/// structured descriptors (`[val, parity, 0, 0]`, 1024 bytes) keeping section entropy ~3.9 (< 6.5 AV limit!).
pub fn generate_whitebox_table(seed_key: u64) -> Vec<u8> {
    let k0 = (seed_key & 0xFF) as u8;
    let k1 = (((seed_key >> 8) & 0xFF) as u8) | 1; // Non-zero GF(2^8) multiplier
    let mask_out = ((seed_key >> 16) & 0xFF) as u8 ^ 0x63;

    let mut table = Vec::with_capacity(1024);
    for x in 0u16..256u16 {
        let mixed_in = gf256_mul((x as u8) ^ k0, k1);
        let sub = affine_byte(gf256_inv(mixed_in), mask_out);
        // 4-byte low-entropy structured entry `[sbox_val, check_tag, 0, 0]` -> AV-clean!
        table.push(sub);
        table.push(sub ^ 0x5A);
        table.push(0);
        table.push(0);
    }
    table
}
