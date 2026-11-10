//! Spec test 3: a naive i64/i128 reference model of the ALU compared with `cpu::alu` on random
//! operands (10⁶ cases in release mode, 10⁵ in debug mode).

use stapelwerk_core::cpu;
use stapelwerk_core::fault::FaultKind;
use stapelwerk_core::isa::{AluOp, FLAG_C, FLAG_N, FLAG_V, FLAG_Z};
use stapelwerk_core::rng::SplitMix64;

const I32_MIN: i128 = i32::MIN as i128;
const I32_MAX: i128 = i32::MAX as i128;
const U32_MAX: i128 = u32::MAX as i128;

fn fits_i32(v: i128) -> bool {
    (I32_MIN..=I32_MAX).contains(&v)
}

/// Builds the flag word from a mathematically exact result and the C/V conditions.
fn flags(result: u32, c: bool, v: bool) -> u32 {
    let mut f = 0;
    if result == 0 {
        f |= FLAG_Z;
    }
    if (result as i32) < 0 {
        f |= FLAG_N;
    }
    if c {
        f |= FLAG_C;
    }
    if v {
        f |= FLAG_V;
    }
    f
}

/// Truncates an exact value to 32 bits (two's complement).
fn wrap(v: i128) -> u32 {
    (v.rem_euclid(1 << 32)) as u32
}

/// Shift one bit at a time, remembering the last bit that fell out.
fn shift_naive(kind: AluOp, a: u32, width: u32) -> (u32, bool) {
    let mut v = a as u64;
    let mut out = false;
    for _ in 0..(width % 32) {
        match kind {
            AluOp::Shl => {
                out = v & 0x8000_0000 != 0;
                v = (v << 1) & 0xFFFF_FFFF;
            }
            AluOp::Shr => {
                out = v & 1 != 0;
                v >>= 1;
            }
            _ => {
                out = v & 1 != 0;
                let sign = v & 0x8000_0000;
                v = (v >> 1) | sign;
            }
        }
    }
    (v as u32, out)
}

/// The reference model: exact arithmetic in i128, flags from their definitions in ISA.md.
fn model(op: AluOp, a: u32, b: u32) -> Result<(u32, u32), FaultKind> {
    let (ua, ub) = (a as i128, b as i128);
    let (sa, sb) = (a as i32 as i128, b as i32 as i128);
    let out = match op {
        AluOp::Add => {
            let r = wrap(ua + ub);
            (r, flags(r, ua + ub > U32_MAX, !fits_i32(sa + sb)))
        }
        AluOp::Sub | AluOp::Cmp => {
            let r = wrap(ua - ub);
            (r, flags(r, ua < ub, !fits_i32(sa - sb)))
        }
        AluOp::Mul => {
            let r = wrap(ua * ub);
            (r, flags(r, ua * ub > U32_MAX, !fits_i32(sa * sb)))
        }
        AluOp::Div | AluOp::Rem => {
            if b == 0 {
                return Err(FaultKind::DivZero);
            }
            // i128 division truncates toward zero; the remainder has the sign of the dividend.
            let q = sa / sb;
            let r = if op == AluOp::Div {
                wrap(q)
            } else {
                wrap(sa - q * sb)
            };
            (r, flags(r, false, !fits_i32(q)))
        }
        AluOp::Divu | AluOp::Remu => {
            if b == 0 {
                return Err(FaultKind::DivZero);
            }
            let r = if op == AluOp::Divu { ua / ub } else { ua % ub };
            let r = r as u32;
            (r, flags(r, false, false))
        }
        AluOp::And | AluOp::Tst => (a & b, flags(a & b, false, false)),
        AluOp::Or => (a | b, flags(a | b, false, false)),
        AluOp::Xor => (a ^ b, flags(a ^ b, false, false)),
        AluOp::Shl | AluOp::Shr | AluOp::Sar => {
            let (r, c) = shift_naive(op, a, b);
            (r, flags(r, c, false))
        }
    };
    Ok(out)
}

/// Operand biased towards interesting values.
fn operand(rng: &mut SplitMix64) -> u32 {
    const EDGES: [u32; 12] = [
        0,
        1,
        2,
        31,
        32,
        33,
        0x7FFF_FFFF,
        0x8000_0000,
        0x8000_0001,
        0xFFFF_FFFF,
        0xFFFF_FFFE,
        0x0001_0000,
    ];
    match rng.below(8) {
        0 => EDGES[rng.below(EDGES.len() as u64) as usize],
        1 => rng.below(64) as u32,
        2 => (rng.below(64) as u32).wrapping_neg(),
        3 => 1u32 << rng.below(32),
        4 => rng.next_u32() & 0xFFFF,
        _ => rng.next_u32(),
    }
}

#[test]
fn model_agrees_with_alu_vectors_spot_checks() {
    // A few hand-checked anchors so a wrong model cannot silently agree with a wrong ALU.
    assert_eq!(
        model(AluOp::Add, 0x7FFF_FFFF, 1),
        Ok((0x8000_0000, FLAG_N | FLAG_V))
    );
    assert_eq!(
        model(AluOp::Rem, 0x8000_0000, 0xFFFF_FFFF),
        Ok((0, FLAG_Z | FLAG_V))
    );
    assert_eq!(
        model(AluOp::Div, 0x8000_0000, 0xFFFF_FFFF),
        Ok((0x8000_0000, FLAG_N | FLAG_V))
    );
    assert_eq!(model(AluOp::Shl, 3, 31), Ok((0x8000_0000, FLAG_N | FLAG_C)));
    assert_eq!(
        model(AluOp::Sar, 0xFFFF_FFF9, 1),
        Ok((0xFFFF_FFFC, FLAG_N | FLAG_C))
    );
    assert_eq!(
        model(AluOp::Mul, 0xFFFF_FFFD, 4),
        Ok((0xFFFF_FFF4, FLAG_N | FLAG_C))
    );
    assert_eq!(model(AluOp::Divu, 1, 0), Err(FaultKind::DivZero));
}

#[test]
fn random_cases_match_the_model() {
    let cases: u32 = if cfg!(debug_assertions) {
        100_000
    } else {
        1_000_000
    };
    let mut rng = SplitMix64::new(0xA1B2_C3D4);
    for i in 0..cases {
        let op = AluOp::ALL[(i % 15) as usize];
        let a = operand(&mut rng);
        let b = operand(&mut rng);
        assert_eq!(
            cpu::alu(op, a, b),
            model(op, a, b),
            "case {i}: {op:?} {a:#010x} {b:#010x}"
        );
    }
}
