//! Runtime faults (`fault.*`). A fault stops the machine; it is *precise*: registers, memory,
//! flags, counters and output are unchanged and `pc` points at the faulting instruction.

use crate::diag::{Arg, Args, write_args};
use crate::json::JsonWriter;

/// Kind of memory access, reported as `"r"` or `"w"` in the `access` argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
}

impl Access {
    /// `"r"` or `"w"`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Access::Read => "r",
            Access::Write => "w",
        }
    }
}

/// What went wrong, with its arguments. Order and numbering follow `docs/ERRORS.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultKind {
    /// Fetched word is not a valid instruction (strict decoding).
    IllegalInstruction { word: u32 },
    /// Fetched word is 0 (usually a missing `halt`).
    ZeroInstruction,
    /// `pc` is outside the occupied part of the text segment.
    ExecOutsideCode,
    /// Jump or call target not 4-aligned or outside the code (reported at the jumping instruction).
    BadJumpTarget { target: u32 },
    /// `ret` popped an address that is not 4-aligned or outside the code.
    BadReturnAddress { target: u32 },
    /// Access to 0x0000…0x00FF.
    NullAccess { addr: u32, access: Access },
    /// Effective address ≥ 0x10000.
    OutOfRange { addr: u32, access: Access },
    /// Access to the guard segment 0xC000…0xCFFF.
    GuardPage { addr: u32, access: Access },
    /// Store into the text segment.
    WriteToCode { addr: u32 },
    /// Word access at an address that is not a multiple of 4 (`size` = 4).
    Misaligned { addr: u32, size: u32 },
    /// `push`/`call` would move `sp` below 0xD000.
    StackOverflow { sp: u32 },
    /// `pop`/`ret` would read at or above 0xF000.
    StackUnderflow { sp: u32 },
    /// `sp` not a multiple of 4 at `push`/`pop`/`call`/`ret`.
    StackMisaligned { sp: u32 },
    /// Division or remainder by zero.
    DivZero,
    /// `READ_INT` with no number left (test mode).
    InputEnd,
    /// Heap access to never-allocated memory (strict heap check).
    HeapUnallocated { addr: u32 },
    /// Heap access into a red zone (strict heap check).
    HeapOverflow { addr: u32 },
    /// Heap access to freed memory (strict heap check).
    UseAfterFree { addr: u32 },
    /// `FREE` of an address that is not the start of a block.
    InvalidFree { addr: u32 },
    /// `FREE` of a block that is already freed.
    DoubleFree { addr: u32 },
    /// `sys n` with an unknown number.
    UnknownSyscall { n: u32 },
    /// `WRITE_STR` found no 0 byte within 4096 bytes.
    StringTooLong { addr: u32 },
    /// The output tape already holds 8192 entries.
    OutputLimit,
}

impl FaultKind {
    /// Number of fault kinds.
    pub const COUNT: u32 = 23;

    /// Key such as `fault.div_zero`.
    pub const fn key(&self) -> &'static str {
        match self {
            FaultKind::IllegalInstruction { .. } => "fault.illegal_instruction",
            FaultKind::ZeroInstruction => "fault.zero_instruction",
            FaultKind::ExecOutsideCode => "fault.exec_outside_code",
            FaultKind::BadJumpTarget { .. } => "fault.bad_jump_target",
            FaultKind::BadReturnAddress { .. } => "fault.bad_return_address",
            FaultKind::NullAccess { .. } => "fault.null_access",
            FaultKind::OutOfRange { .. } => "fault.out_of_range",
            FaultKind::GuardPage { .. } => "fault.guard_page",
            FaultKind::WriteToCode { .. } => "fault.write_to_code",
            FaultKind::Misaligned { .. } => "fault.misaligned",
            FaultKind::StackOverflow { .. } => "fault.stack_overflow",
            FaultKind::StackUnderflow { .. } => "fault.stack_underflow",
            FaultKind::StackMisaligned { .. } => "fault.stack_misaligned",
            FaultKind::DivZero => "fault.div_zero",
            FaultKind::InputEnd => "fault.input_end",
            FaultKind::HeapUnallocated { .. } => "fault.heap_unallocated",
            FaultKind::HeapOverflow { .. } => "fault.heap_overflow",
            FaultKind::UseAfterFree { .. } => "fault.use_after_free",
            FaultKind::InvalidFree { .. } => "fault.invalid_free",
            FaultKind::DoubleFree { .. } => "fault.double_free",
            FaultKind::UnknownSyscall { .. } => "fault.unknown_syscall",
            FaultKind::StringTooLong { .. } => "fault.string_too_long",
            FaultKind::OutputLimit => "fault.output_limit",
        }
    }

    /// Numeric fault code for `StateView.fault_code` (1…23; 0 means "no fault").
    pub const fn code(&self) -> u32 {
        match self {
            FaultKind::IllegalInstruction { .. } => 1,
            FaultKind::ZeroInstruction => 2,
            FaultKind::ExecOutsideCode => 3,
            FaultKind::BadJumpTarget { .. } => 4,
            FaultKind::BadReturnAddress { .. } => 5,
            FaultKind::NullAccess { .. } => 6,
            FaultKind::OutOfRange { .. } => 7,
            FaultKind::GuardPage { .. } => 8,
            FaultKind::WriteToCode { .. } => 9,
            FaultKind::Misaligned { .. } => 10,
            FaultKind::StackOverflow { .. } => 11,
            FaultKind::StackUnderflow { .. } => 12,
            FaultKind::StackMisaligned { .. } => 13,
            FaultKind::DivZero => 14,
            FaultKind::InputEnd => 15,
            FaultKind::HeapUnallocated { .. } => 16,
            FaultKind::HeapOverflow { .. } => 17,
            FaultKind::UseAfterFree { .. } => 18,
            FaultKind::InvalidFree { .. } => 19,
            FaultKind::DoubleFree { .. } => 20,
            FaultKind::UnknownSyscall { .. } => 21,
            FaultKind::StringTooLong { .. } => 22,
            FaultKind::OutputLimit => 23,
        }
    }

    /// The first numeric argument (`word`, `target`, `addr`, `sp` or `n`), or 0.
    /// Exposed as `StateView.fault_addr`.
    pub const fn addr(&self) -> u32 {
        match *self {
            FaultKind::IllegalInstruction { word } => word,
            FaultKind::BadJumpTarget { target } | FaultKind::BadReturnAddress { target } => target,
            FaultKind::NullAccess { addr, .. }
            | FaultKind::OutOfRange { addr, .. }
            | FaultKind::GuardPage { addr, .. }
            | FaultKind::WriteToCode { addr }
            | FaultKind::Misaligned { addr, .. }
            | FaultKind::HeapUnallocated { addr }
            | FaultKind::HeapOverflow { addr }
            | FaultKind::UseAfterFree { addr }
            | FaultKind::InvalidFree { addr }
            | FaultKind::DoubleFree { addr }
            | FaultKind::StringTooLong { addr } => addr,
            FaultKind::StackOverflow { sp }
            | FaultKind::StackUnderflow { sp }
            | FaultKind::StackMisaligned { sp } => sp,
            FaultKind::UnknownSyscall { n } => n,
            FaultKind::ZeroInstruction
            | FaultKind::ExecOutsideCode
            | FaultKind::DivZero
            | FaultKind::InputEnd
            | FaultKind::OutputLimit => 0,
        }
    }

    /// Named arguments in the order of `docs/ERRORS.md`.
    pub fn args(&self) -> Args {
        let a = |v: u32| Arg::Int(v as i64);
        match *self {
            FaultKind::IllegalInstruction { word } => vec![("word", a(word))],
            FaultKind::BadJumpTarget { target } | FaultKind::BadReturnAddress { target } => {
                vec![("target", a(target))]
            }
            FaultKind::NullAccess { addr, access }
            | FaultKind::OutOfRange { addr, access }
            | FaultKind::GuardPage { addr, access } => {
                vec![("addr", a(addr)), ("access", Arg::from(access.as_str()))]
            }
            FaultKind::Misaligned { addr, size } => vec![("addr", a(addr)), ("size", a(size))],
            FaultKind::WriteToCode { addr }
            | FaultKind::HeapUnallocated { addr }
            | FaultKind::HeapOverflow { addr }
            | FaultKind::UseAfterFree { addr }
            | FaultKind::InvalidFree { addr }
            | FaultKind::DoubleFree { addr }
            | FaultKind::StringTooLong { addr } => vec![("addr", a(addr))],
            FaultKind::StackOverflow { sp }
            | FaultKind::StackUnderflow { sp }
            | FaultKind::StackMisaligned { sp } => vec![("sp", a(sp))],
            FaultKind::UnknownSyscall { n } => vec![("n", a(n))],
            FaultKind::ZeroInstruction
            | FaultKind::ExecOutsideCode
            | FaultKind::DivZero
            | FaultKind::InputEnd
            | FaultKind::OutputLimit => Vec::new(),
        }
    }

    /// Attaches the location.
    pub const fn at(self, pc: u32, step: u64) -> Fault {
        Fault {
            kind: self,
            pc,
            step,
        }
    }
}

/// A fault with its location: `pc` of the faulting instruction and `step` = number of
/// instructions completed before it (the step counter is not incremented by a fault).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fault {
    pub kind: FaultKind,
    pub pc: u32,
    pub step: u64,
}

impl Fault {
    /// Key of the kind.
    pub const fn key(&self) -> &'static str {
        self.kind.key()
    }

    /// Writes `{"key","pc","step","args":{…}}`.
    pub fn write_json(&self, w: &mut JsonWriter) {
        w.begin_object();
        w.field_str("key", self.kind.key());
        w.field_uint("pc", self.pc as u64);
        w.field_uint("step", self.step);
        w.key("args");
        write_args(w, &self.kind.args());
        w.end_object();
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::diag::key_args;

    /// One example of every fault kind, in code order.
    pub(crate) fn all_kinds() -> [FaultKind; FaultKind::COUNT as usize] {
        [
            FaultKind::IllegalInstruction { word: 0x0300_0000 },
            FaultKind::ZeroInstruction,
            FaultKind::ExecOutsideCode,
            FaultKind::BadJumpTarget { target: 0x4000 },
            FaultKind::BadReturnAddress { target: 2 },
            FaultKind::NullAccess {
                addr: 4,
                access: Access::Read,
            },
            FaultKind::OutOfRange {
                addr: 0x1_0000,
                access: Access::Write,
            },
            FaultKind::GuardPage {
                addr: 0xC000,
                access: Access::Write,
            },
            FaultKind::WriteToCode { addr: 0x0100 },
            FaultKind::Misaligned {
                addr: 0x4001,
                size: 4,
            },
            FaultKind::StackOverflow { sp: 0xD000 },
            FaultKind::StackUnderflow { sp: 0xF000 },
            FaultKind::StackMisaligned { sp: 0xEFFE },
            FaultKind::DivZero,
            FaultKind::InputEnd,
            FaultKind::HeapUnallocated { addr: 0x8000 },
            FaultKind::HeapOverflow { addr: 0x8008 },
            FaultKind::UseAfterFree { addr: 0x8000 },
            FaultKind::InvalidFree { addr: 0x8004 },
            FaultKind::DoubleFree { addr: 0x8000 },
            FaultKind::UnknownSyscall { n: 99 },
            FaultKind::StringTooLong { addr: 0x4000 },
            FaultKind::OutputLimit,
        ]
    }

    #[test]
    fn keys_codes_and_args_match_the_key_list() {
        for (i, kind) in all_kinds().iter().enumerate() {
            assert_eq!(kind.code(), i as u32 + 1);
            let expected = key_args(kind.key()).expect("key listed in diag::ALL_KEYS");
            let names: Vec<&str> = kind.args().iter().map(|(n, _)| *n).collect();
            assert_eq!(names, expected, "{}", kind.key());
            assert_eq!(crate::diag::ALL_KEYS[i].0, kind.key());
        }
    }

    #[test]
    fn json_shape() {
        let f = FaultKind::NullAccess {
            addr: 8,
            access: Access::Write,
        }
        .at(0x104, 7);
        let mut w = JsonWriter::new();
        f.write_json(&mut w);
        assert_eq!(
            w.finish(),
            r#"{"key":"fault.null_access","pc":260,"step":7,"args":{"addr":8,"access":"w"}}"#
        );
    }
}
