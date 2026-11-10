# Stapelwerk

Assembly puzzle game in the browser: a 32-bit teaching CPU in Rust (zero third-party crates) compiled to WebAssembly, a vanilla-JS interface without bundler or npm packages, 30 levels in German and English. Read `README.md` for the overview and `CONTRIBUTING.md` for the full rules.

## Commands

- Setup: rustup and Node >= 22. `rust-toolchain.toml` pins Rust 1.99.1 with the `wasm32-unknown-unknown` target. Nothing to `npm install`.
- `npm start`: build and serve `http://127.0.0.1:4860/stapelwerk/` (English: `/stapelwerk/en/`).
- `npm test`: `cargo test --workspace` plus the Node tests in `test/unit`, `test/wasm`, `test/site` (`test/wasm` needs `npm run build` first).
- `npm run test:browser`: headless Chrome over CDP (`CHROME_PATH` if not found).
- `npm run check`: everything CI runs, with a summary. Check the exit code.
- Levels: `cargo run -p stapelwerk-cli -- check-levels --strict --mutate levels/`; one program against a level: `cargo run -q -p stapelwerk-cli -- run prog.s --level levels/<id>.lvl [--test NAME] [--trace]`.
- After changing `crates/core/src/isa.rs`: `cargo run -q -p stapelwerk-cli -- isa-table > docs/isa-table.md`.
- On small machines: `export CARGO_BUILD_JOBS=2`. The exhaustive 2³² test is `#[ignore]` and runs only in CI.

## Where the contract is

- `docs/ISA.md` (machine and assembly language; `crates/core/src/isa.rs` is the single source for opcodes, generated into `docs/isa-table.md`)
- `docs/ABI.md` and `docs/abi-exports.txt` (WebAssembly interface between `crates/wasm` and `web/js/vm.js`)
- `docs/LEVELS.md` (level file format, test semantics, CLI) and `docs/CURRICULUM.md` (chapters, content rules)
- `docs/ERRORS.md` (every error key; must match `diag::ALL_KEYS`)

These files change only with a reason, and then code, tests and docs change together.

## Rules

- No third-party crates (also not as dev-dependencies), no npm packages, no CDN.
- Every interface text in both `web/js/texts/de.js` and `web/js/texts/en.js` with the same keys and placeholders; every level text in `.de` and `.en`.
- No requests to other servers; keep the Content-Security-Policy in `web/index.html` strict. No inline scripts or styles, no `innerHTML`: DOM only through `textContent`/`setAttribute`.
- The core stays deterministic: no floats, no clocks, no `HashMap`/`HashSet`, no panic from player input. `unsafe` only in `crates/wasm`.
- `web/js/vm.js` is the only module that touches WebAssembly.
- Code comments in English. German level texts use "du"; statements about real CPUs must be correct (`docs/TO-REAL-CPUS.md`).
- Public repository: no private paths, e-mail addresses or keys in any file.
