//! ALU vectors from `tests/vectors/alu.txt` (spec test 2).

use stapelwerk_core::cpu;
use stapelwerk_core::fault::FaultKind;
use stapelwerk_core::isa::{AluOp, FLAG_C, FLAG_N, FLAG_V, FLAG_Z};

const VECTORS: &str = include_str!("../../../tests/vectors/alu.txt");

#[derive(Debug)]
struct Vector {
    line: usize,
    op: AluOp,
    a: u32,
    b: u32,
    /// `None` = `fault.div_zero`.
    expected: Option<(u32, u32)>,
}

fn number(t: &str) -> Option<u32> {
    if let Some(hex) = t.strip_prefix("0x") {
        u32::from_str_radix(hex, 16).ok()
    } else {
        t.parse::<i64>()
            .ok()
            .filter(|v| (-(1i64 << 31)..(1i64 << 32)).contains(v))
            .map(|v| v as u32)
    }
}

fn flags(t: &str) -> Option<u32> {
    if t == "-" {
        return Some(0);
    }
    let mut f = 0;
    let mut last = 0;
    for ch in t.chars() {
        let (bit, order) = match ch {
            'Z' => (FLAG_Z, 1),
            'N' => (FLAG_N, 2),
            'C' => (FLAG_C, 3),
            'V' => (FLAG_V, 4),
            _ => return None,
        };
        if order <= last {
            return None; // letters must be in the order Z N C V
        }
        last = order;
        f |= bit;
    }
    Some(f)
}

fn parse() -> Vec<Vector> {
    let mut out = Vec::new();
    for (i, raw) in VECTORS.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (lhs, rhs) = line
            .split_once("->")
            .unwrap_or_else(|| panic!("line {}: missing '->'", i + 1));
        let lhs: Vec<&str> = lhs.split_whitespace().collect();
        let rhs: Vec<&str> = rhs.split_whitespace().collect();
        assert_eq!(lhs.len(), 3, "line {}: expected 'op a b'", i + 1);
        let op = AluOp::ALL
            .iter()
            .copied()
            .find(|o| o.mnemonic() == lhs[0])
            .unwrap_or_else(|| panic!("line {}: unknown op {}", i + 1, lhs[0]));
        let a = number(lhs[1]).unwrap_or_else(|| panic!("line {}: bad a", i + 1));
        let b = number(lhs[2]).unwrap_or_else(|| panic!("line {}: bad b", i + 1));
        let expected = if rhs == ["fault.div_zero"] {
            None
        } else {
            assert_eq!(rhs.len(), 2, "line {}: expected 'result FLAGS'", i + 1);
            assert!(
                rhs[0].starts_with("0x") && rhs[0].len() == 10,
                "line {}: result must be 0x + 8 hex digits",
                i + 1
            );
            let r = number(rhs[0]).unwrap_or_else(|| panic!("line {}: bad result", i + 1));
            let f = flags(rhs[1]).unwrap_or_else(|| panic!("line {}: bad flags", i + 1));
            Some((r, f))
        };
        out.push(Vector {
            line: i + 1,
            op,
            a,
            b,
            expected,
        });
    }
    out
}

#[test]
fn vectors_parse_and_cover_every_op() {
    let v = parse();
    assert!(v.len() >= 60, "only {} vectors", v.len());
    for op in AluOp::ALL {
        assert!(v.iter().any(|x| x.op == op), "no vector for {op:?}");
    }
    for op in [AluOp::Div, AluOp::Divu, AluOp::Rem, AluOp::Remu] {
        assert!(
            v.iter().any(|x| x.op == op && x.expected.is_none()),
            "no division-by-zero vector for {op:?}"
        );
    }
}

#[test]
fn vectors_match_cpu_alu() {
    for v in parse() {
        let got = cpu::alu(v.op, v.a, v.b);
        match v.expected {
            Some((r, f)) => assert_eq!(got, Ok((r, f)), "alu.txt line {}: {v:?}", v.line),
            None => assert_eq!(
                got,
                Err(FaultKind::DivZero),
                "alu.txt line {}: {v:?}",
                v.line
            ),
        }
    }
}
