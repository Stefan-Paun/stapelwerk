//! WebAssembly ABI v1 of Stapelwerk (`docs/ABI.md`, exports listed in `docs/abi-exports.txt`).
//!
//! Built as a `cdylib` for `wasm32-unknown-unknown` with **no imports**. Every export is a plain
//! `extern "C"` function that takes and returns only `i32`/`u32`. This crate is the only place in
//! the workspace that may contain `unsafe`; in fact the only unsafe construct is the
//! `#[unsafe(no_mangle)]` attribute. Memory shared with JavaScript is handled without raw pointer
//! access:
//!
//! - input buffers (`buf_alloc`) are boxed slices registered here; functions that take
//!   `(ptr, len)` look the range up in that registry and copy it (an unknown range is `abi.bad_arg`),
//! - pointers handed out (`out_ptr`, `sess_view`, `sess_mem`, …) are addresses of buffers owned by
//!   this crate or the core; JavaScript reads them through `memory`.
//!
//! All state lives in one `thread_local!` [`RefCell`]. Levels and sessions share one handle table
//! ([`table::Table`]), so a level handle passed to a `sess_*` function is `abi.bad_handle`.

mod render;
mod table;

use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

use stapelwerk_core::consts::MEM_SIZE;
use stapelwerk_core::json::JsonWriter;
use stapelwerk_core::level::{self, IMAGE_SECTION_PREFIX, Level, ScreenExpect, image_rows};
use stapelwerk_core::runner::{self, Reference, ScreenRef};
use stapelwerk_core::session::{ChangeKind, Session, SessionConfig, StateView};

use table::Table;

/// Status / pointer / count result for handle and argument errors.
pub const ERR: u32 = u32::MAX;

const BAD_HANDLE: &str = "abi.bad_handle";
const BAD_ARG: &str = "abi.bad_arg";
const NOT_ASSEMBLED: &str = "abi.not_assembled";

/// Upper limit of `sess_disasm`'s `count`.
const MAX_DISASM: u32 = 4096;

/// A loaded level and its lazily computed reference run (unpacked levels only).
struct LevelData {
    level: Level,
    reference: OnceCell<Option<Reference>>,
}

impl LevelData {
    /// The reference solution run on every test; `None` for packed levels (no `[reference]`) or
    /// a reference that does not assemble. Computed once per level.
    fn reference(&self) -> Option<&Reference> {
        self.reference
            .get_or_init(|| runner::run_reference(&self.level).ok().flatten())
            .as_ref()
    }

    /// Reference screen after test `index`, if there is a reference.
    fn reference_screen(&self, index: u32) -> Option<&[u8]> {
        self.reference()
            .and_then(|r| r.screens.get(index as usize))
            .map(Vec::as_slice)
            .filter(|s| !s.is_empty())
    }

    /// The screen source for evaluating test `index` (only computes the reference when the test
    /// compares against it).
    fn screen_ref(&self, index: u32) -> ScreenRef<'_> {
        let wants = self
            .level
            .tests
            .get(index as usize)
            .is_some_and(|t| t.expect_screen == Some(ScreenExpect::Reference));
        match self.reference_screen(index) {
            Some(s) if wants => ScreenRef::Screen(s),
            _ => ScreenRef::None,
        }
    }
}

/// A debugger session plus its level and the exported `StateView`.
struct Sess {
    session: Session,
    level: Option<Rc<LevelData>>,
    /// Boxed so its address is stable (`sess_view`); refreshed after every session call.
    view: Box<StateView>,
}

impl Sess {
    fn refresh(&mut self) {
        *self.view = self.session.view();
    }
}

enum Entry {
    Level(Rc<LevelData>),
    Session(Box<Sess>),
}

#[derive(Default)]
struct State {
    table: Table<Entry>,
    /// Global output buffer (`out_ptr`, `out_len`).
    out: Vec<u8>,
    /// Live input buffers from `buf_alloc`.
    bufs: Vec<Box<[u8]>>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

/// Address of a buffer in linear memory (32-bit on wasm32; truncated in native unit tests).
fn addr<T>(p: *const T) -> u32 {
    p as usize as u32
}

impl State {
    /// Replaces the output buffer; returns the length.
    fn set_out(&mut self, text: String) -> u32 {
        self.out = text.into_bytes();
        self.out.len() as u32
    }

    /// Copies `len` bytes at `ptr` out of a live input buffer; `None` if the range does not lie
    /// inside one (`len = 0` is always valid).
    fn input(&self, ptr: u32, len: u32) -> Option<Vec<u8>> {
        if len == 0 {
            return Some(Vec::new());
        }
        self.bufs.iter().find_map(|b| {
            let start = ptr.checked_sub(addr(b.as_ptr()))? as usize;
            let end = start.checked_add(len as usize)?;
            b.get(start..end).map(<[u8]>::to_vec)
        })
    }

    fn level(&self, lv: u32) -> Option<&Rc<LevelData>> {
        match self.table.get(lv) {
            Some(Entry::Level(l)) => Some(l),
            _ => None,
        }
    }

    fn sess(&mut self, s: u32) -> Option<&mut Sess> {
        match self.table.get_mut(s) {
            Some(Entry::Session(x)) => Some(x),
            _ => None,
        }
    }
}

/// Session call with a status / pointer / count result (`ERR` for an invalid handle).
fn sess_u32(s: u32, f: impl FnOnce(&mut Sess) -> u32) -> u32 {
    with(|st| match st.sess(s) {
        Some(x) => {
            let r = f(x);
            x.refresh();
            r
        }
        None => ERR,
    })
}

/// Session call with a JSON (or raw text) result in the output buffer; errors become
/// `{"error":"abi.…"}`. Returns the length.
fn sess_json(s: u32, f: impl FnOnce(&mut Sess) -> Result<String, &'static str>) -> u32 {
    with(|st| {
        let res = match st.sess(s) {
            Some(x) => {
                let r = f(x);
                x.refresh();
                r
            }
            None => Err(BAD_HANDLE),
        };
        st.set_out(res.unwrap_or_else(render::error))
    })
}

/// Status code of a session result (`SessionError` → `ERR`).
fn status_of<E>(r: Result<stapelwerk_core::machine::Status, E>) -> u32 {
    r.map_or(ERR, |s| s.code())
}

// -------------------------------------------------------------------------------------------
// Module, buffers
// -------------------------------------------------------------------------------------------

/// ABI version. JavaScript (`web/js/vm.js`) refuses to run with any other value.
#[unsafe(no_mangle)]
pub extern "C" fn abi_version() -> u32 {
    1
}

/// Allocates an input buffer of `len` bytes (zeroed); 0 if `len` is 0 or allocation fails.
#[unsafe(no_mangle)]
pub extern "C" fn buf_alloc(len: u32) -> u32 {
    if len == 0 {
        return 0;
    }
    let mut v: Vec<u8> = Vec::new();
    if v.try_reserve_exact(len as usize).is_err() {
        return 0;
    }
    v.resize(len as usize, 0);
    let b = v.into_boxed_slice();
    let p = addr(b.as_ptr());
    with(|st| st.bufs.push(b));
    p
}

/// Frees an input buffer from [`buf_alloc`]; unknown `(ptr, len)` pairs are ignored.
#[unsafe(no_mangle)]
pub extern "C" fn buf_free(ptr: u32, len: u32) {
    with(|st| {
        if let Some(i) = st
            .bufs
            .iter()
            .position(|b| addr(b.as_ptr()) == ptr && b.len() == len as usize)
        {
            st.bufs.swap_remove(i);
        }
    });
}

/// Address of the output buffer.
#[unsafe(no_mangle)]
pub extern "C" fn out_ptr() -> u32 {
    with(|st| addr(st.out.as_ptr()))
}

/// Length of the last result in the output buffer.
#[unsafe(no_mangle)]
pub extern "C" fn out_len() -> u32 {
    with(|st| st.out.len() as u32)
}

/// JSON memory map.
#[unsafe(no_mangle)]
pub extern "C" fn segments() -> u32 {
    with(|st| st.set_out(render::segments()))
}

// -------------------------------------------------------------------------------------------
// Levels
// -------------------------------------------------------------------------------------------

/// Parses a `.lvl` file. Returns the level handle, or 0 with `{"ok":false,"diagnostics":[…]}`
/// (or `{"error":"abi.bad_arg"}` for an invalid buffer) in the output buffer.
#[unsafe(no_mangle)]
pub extern "C" fn level_load(ptr: u32, len: u32) -> u32 {
    with(|st| {
        let Some(bytes) = st.input(ptr, len) else {
            st.set_out(render::error(BAD_ARG));
            return 0;
        };
        match level::parse(&String::from_utf8_lossy(&bytes)) {
            Ok(level) => {
                let data = Rc::new(LevelData {
                    level,
                    reference: OnceCell::new(),
                });
                match st.table.insert(Entry::Level(data)) {
                    Some(h) => h,
                    None => {
                        st.set_out(render::error(BAD_ARG));
                        0
                    }
                }
            }
            Err(diags) => {
                st.set_out(render::level_error(&diags));
                0
            }
        }
    })
}

/// Releases a level (sessions created from it keep their copy).
#[unsafe(no_mangle)]
pub extern "C" fn level_free(lv: u32) {
    with(|st| {
        st.table.remove_if(lv, |e| matches!(e, Entry::Level(_)));
    });
}

/// JSON level summary.
#[unsafe(no_mangle)]
pub extern "C" fn level_info(lv: u32) -> u32 {
    with(|st| {
        let text = match st.level(lv) {
            Some(l) => l.level.info_json(),
            None => render::error(BAD_HANDLE),
        };
        st.set_out(text)
    })
}

/// Raw text of a section in the output buffer; −1 if missing. `image.NAME` returns the image as
/// rows (`.` and hex digits, joined with `\n`); `reference` is never returned.
#[unsafe(no_mangle)]
pub extern "C" fn level_section(lv: u32, name_ptr: u32, name_len: u32) -> i32 {
    with(|st| {
        let Some(name) = st.input(name_ptr, name_len) else {
            st.set_out(render::error(BAD_ARG));
            return -1;
        };
        let name = String::from_utf8_lossy(&name);
        let Some(l) = st.level(lv) else {
            st.set_out(render::error(BAD_HANDLE));
            return -1;
        };
        let level = &l.level;
        let text = match name.strip_prefix(IMAGE_SECTION_PREFIX) {
            Some(img) => level.image(img).map(|i| image_rows(&i.pixels).join("\n")),
            None if name == "reference" => None,
            None => level.section(&name).map(str::to_string),
        };
        match text {
            Some(t) => st.set_out(t) as i32,
            None => -1,
        }
    })
}

// -------------------------------------------------------------------------------------------
// Sessions: lifetime, program, execution
// -------------------------------------------------------------------------------------------

fn session_config(level: &Level) -> SessionConfig {
    SessionConfig {
        library: level.section("library").map(str::to_string),
        filter: level.filter.clone(),
        heap_check: level.heap_check,
        seed: level.seed,
        budget: level.budget,
    }
}

/// New session for level `lv`; `lv = 0` creates a sandbox session. `0xFFFFFFFF` for an invalid
/// level handle or a full handle table.
#[unsafe(no_mangle)]
pub extern "C" fn sess_new(lv: u32) -> u32 {
    with(|st| {
        let level = if lv == 0 {
            None
        } else {
            match st.level(lv) {
                Some(l) => Some(Rc::clone(l)),
                None => return ERR,
            }
        };
        let session = match &level {
            Some(l) => Session::new(session_config(&l.level)),
            None => Session::sandbox(),
        };
        let view = Box::new(session.view());
        let sess = Box::new(Sess {
            session,
            level,
            view,
        });
        st.table.insert(Entry::Session(sess)).unwrap_or(ERR)
    })
}

/// Releases a session.
#[unsafe(no_mangle)]
pub extern "C" fn sess_free(s: u32) {
    with(|st| {
        st.table.remove_if(s, |e| matches!(e, Entry::Session(_)));
    });
}

/// Assembles the player's code. Returns the number of diagnostics (0 = success) and writes
/// `{ok, size, diagnostics}`.
#[unsafe(no_mangle)]
pub extern "C" fn sess_assemble(s: u32, ptr: u32, len: u32) -> u32 {
    with(|st| {
        let Some(bytes) = st.input(ptr, len) else {
            st.set_out(render::error(BAD_ARG));
            return ERR;
        };
        let Some(x) = st.sess(s) else {
            st.set_out(render::error(BAD_HANDLE));
            return ERR;
        };
        let source = String::from_utf8_lossy(&bytes);
        let (count, json) = match x.session.assemble(&source) {
            Ok(()) => {
                let size = x.session.program().map_or(0, |p| p.size);
                (0, render::assembled(true, size, &[]))
            }
            Err(diags) => (diags.len() as u32, render::assembled(false, 0, &diags)),
        };
        x.refresh();
        st.set_out(json);
        count
    })
}

/// Fresh machine for test `test`, or −1 for an interactive run.
#[unsafe(no_mangle)]
pub extern "C" fn sess_reset(s: u32, test: i32) -> u32 {
    sess_u32(s, |x| {
        let setup = if test == -1 {
            None
        } else {
            let (Some(lv), Ok(i)) = (&x.level, u32::try_from(test)) else {
                return ERR;
            };
            let Some(t) = lv.level.tests.get(i as usize) else {
                return ERR;
            };
            Some(t.to_setup(&lv.level, i))
        };
        x.session.reset(setup).code()
    })
}

/// Executes up to `n` instructions, ignoring breakpoints.
#[unsafe(no_mangle)]
pub extern "C" fn sess_step(s: u32, n: u32) -> u32 {
    sess_u32(s, |x| x.session.step(n as u64).code())
}

/// Runs one time slice of at most `max_cycles` cycles; stops at breakpoints.
#[unsafe(no_mangle)]
pub extern "C" fn sess_run(s: u32, max_cycles: u32) -> u32 {
    sess_u32(s, |x| x.session.run(max_cycles as u64).code())
}

/// Goes back `n` steps.
#[unsafe(no_mangle)]
pub extern "C" fn sess_back(s: u32, n: u32) -> u32 {
    sess_u32(s, |x| x.session.back(n as u64).code())
}

/// Restores the state after exactly `step` steps.
#[unsafe(no_mangle)]
pub extern "C" fn sess_goto(s: u32, step: u32) -> u32 {
    sess_u32(s, |x| x.session.goto(step as u64).code())
}

/// Goes to just before the last change of a register (`kind` 0), word (1) or byte (2).
#[unsafe(no_mangle)]
pub extern "C" fn sess_back_to_change(s: u32, kind: u32, target: u32) -> u32 {
    sess_u32(s, |x| match ChangeKind::from_code(kind) {
        Some(k) => status_of(x.session.back_to_change(k, target)),
        None => ERR,
    })
}

/// Edits a register (0–9, 10 = flags, 11 = pc) while the run can continue.
#[unsafe(no_mangle)]
pub extern "C" fn sess_set_reg(s: u32, reg: u32, value: u32) -> u32 {
    sess_u32(s, |x| status_of(x.session.set_reg(reg, value)))
}

/// Sets (`on = 1`) or clears (`on = 0`) a breakpoint; 1 on success, 0 if `addr` is not an
/// instruction address.
#[unsafe(no_mangle)]
pub extern "C" fn sess_bp(s: u32, addr: u32, on: u32) -> u32 {
    sess_u32(s, |x| match on {
        0 | 1 => x.session.set_breakpoint(addr, on == 1) as u32,
        _ => ERR,
    })
}

/// Clears all breakpoints.
#[unsafe(no_mangle)]
pub extern "C" fn sess_bp_clear(s: u32) {
    sess_u32(s, |x| {
        x.session.clear_breakpoints();
        0
    });
}

/// Appends a number to the interactive input.
#[unsafe(no_mangle)]
pub extern "C" fn sess_in_int(s: u32, v: i32) -> u32 {
    sess_u32(s, |x| x.session.push_int(v).code())
}

/// Appends bytes to the interactive text input.
#[unsafe(no_mangle)]
pub extern "C" fn sess_in_text(s: u32, ptr: u32, len: u32) -> u32 {
    with(|st| {
        let Some(bytes) = st.input(ptr, len) else {
            return ERR;
        };
        match st.sess(s) {
            Some(x) => {
                let r = x.session.push_text(&bytes).code();
                x.refresh();
                r
            }
            None => ERR,
        }
    })
}

/// Closes the interactive input.
#[unsafe(no_mangle)]
pub extern "C" fn sess_in_close(s: u32) -> u32 {
    sess_u32(s, |x| x.session.close_input().code())
}

// -------------------------------------------------------------------------------------------
// Sessions: stable views
// -------------------------------------------------------------------------------------------

/// Pointer to the 96-byte `StateView`.
#[unsafe(no_mangle)]
pub extern "C" fn sess_view(s: u32) -> u32 {
    sess_u32(s, |x| addr(&*x.view))
}

/// Pointer to the 65536 memory bytes.
#[unsafe(no_mangle)]
pub extern "C" fn sess_mem(s: u32) -> u32 {
    sess_u32(s, |x| addr(x.session.memory().as_ptr()))
}

/// Pointer to the 4096-byte heap shadow.
#[unsafe(no_mangle)]
pub extern "C" fn sess_shadow(s: u32) -> u32 {
    sess_u32(s, |x| addr(x.session.shadow().as_ptr()))
}

/// Pointer to `u8[8192]` output kinds.
#[unsafe(no_mangle)]
pub extern "C" fn sess_out_kinds(s: u32) -> u32 {
    sess_u32(s, |x| addr(x.session.tapes().out_kinds.as_ptr()))
}

/// Pointer to `i32[8192]` output values.
#[unsafe(no_mangle)]
pub extern "C" fn sess_out_vals(s: u32) -> u32 {
    sess_u32(s, |x| addr(x.session.tapes().out_vals.as_ptr()))
}

// -------------------------------------------------------------------------------------------
// Sessions: JSON queries
// -------------------------------------------------------------------------------------------

/// `{key, pc, step, args}` or `null`.
#[unsafe(no_mangle)]
pub extern "C" fn sess_fault(s: u32) -> u32 {
    sess_json(s, |x| {
        let mut w = JsonWriter::new();
        match x.session.fault() {
            Some(f) => f.write_json(&mut w),
            None => w.null(),
        }
        Ok(w.finish())
    })
}

/// `[{name, kind, value, file, line}]`.
#[unsafe(no_mangle)]
pub extern "C" fn sess_symbols(s: u32) -> u32 {
    sess_json(s, |x| Ok(render::symbols(x.session.symbols())))
}

/// `[{a, f, l}]`.
#[unsafe(no_mangle)]
pub extern "C" fn sess_linemap(s: u32) -> u32 {
    sess_json(s, |x| Ok(render::linemap(x.session.linemap())))
}

/// `[[line, count], …]` for file 0.
#[unsafe(no_mangle)]
pub extern "C" fn sess_line_counts(s: u32) -> u32 {
    sess_json(s, |x| Ok(render::line_counts(&x.session.line_counts())))
}

/// `[{fp, ret, func, size}]`, innermost first.
#[unsafe(no_mangle)]
pub extern "C" fn sess_frames(s: u32) -> u32 {
    sess_json(s, |x| Ok(render::frames(&x.session.frames())))
}

/// `[{addr, size, live, line}]`.
#[unsafe(no_mangle)]
pub extern "C" fn sess_heap(s: u32) -> u32 {
    sess_json(s, |x| Ok(render::heap(&x.session.heap_blocks())))
}

/// `[{addr, word, text, label}]` for `count` (≤ 4096) words from `addr` (< 0x10000).
#[unsafe(no_mangle)]
pub extern "C" fn sess_disasm(s: u32, addr: u32, count: u32) -> u32 {
    sess_json(s, |x| {
        if addr >= MEM_SIZE || count > MAX_DISASM {
            return Err(BAD_ARG);
        }
        Ok(render::disasm(&x.session.disasm(addr, count)))
    })
}

/// Evaluates the current test (one `run_tests` entry); `null` for an interactive run or a
/// sandbox session.
#[unsafe(no_mangle)]
pub extern "C" fn sess_check(s: u32) -> u32 {
    sess_json(s, |x| {
        let Some(program) = x.session.program() else {
            return Err(NOT_ASSEMBLED);
        };
        let (Some(lv), Ok(index)) = (&x.level, u32::try_from(x.session.test_index())) else {
            return Ok("null".to_string());
        };
        let (result, _) = runner::run_test_machine(&lv.level, program, index, lv.screen_ref(index));
        let mut w = JsonWriter::new();
        result.write_json_with_input(&lv.level, &mut w);
        Ok(w.finish())
    })
}

/// Runs all tests of the level (a sandbox session reports an empty list).
#[unsafe(no_mangle)]
pub extern "C" fn sess_run_tests(s: u32) -> u32 {
    sess_json(s, |x| {
        let Some(program) = x.session.program() else {
            return Err(NOT_ASSEMBLED);
        };
        Ok(match &x.level {
            Some(lv) => {
                runner::run_tests_with(&lv.level, program, lv.reference()).to_json(&lv.level)
            }
            None => {
                let empty = Level::default();
                runner::run_tests_with(&empty, program, None).to_json(&empty)
            }
        })
    })
}

/// Pointer to 4096 bytes: the reference solution's screen after `test`; 0 if the level has no
/// reference (packed levels, sandbox).
#[unsafe(no_mangle)]
pub extern "C" fn sess_reference_screen(s: u32, test: i32) -> u32 {
    sess_u32(s, |x| {
        let Some(lv) = &x.level else {
            return 0;
        };
        let Some(index) = u32::try_from(test)
            .ok()
            .filter(|&i| (i as usize) < lv.level.tests.len())
        else {
            return ERR;
        };
        lv.reference_screen(index)
            .map_or(0, |scr| addr(scr.as_ptr()))
    })
}

/// 16 lower-case hex characters (raw ASCII).
#[unsafe(no_mangle)]
pub extern "C" fn sess_digest(s: u32) -> u32 {
    sess_json(s, |x| Ok(x.session.digest()))
}

#[cfg(test)]
mod tests;
