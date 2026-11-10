# Contributing to Stapelwerk

Thanks for wanting to help. Small, focused contributions are the easiest to review. For bugs and level ideas there are issue templates; for anything bigger, please open an issue first so we can agree on the direction. Security problems go through [SECURITY.md](SECURITY.md), not public issues.

## Setup

You need [rustup](https://rustup.rs) and Node 22 or newer. There are no npm packages and no third-party crates, so there is nothing to install beyond that. `rust-toolchain.toml` pins Rust 1.99.1 with `rustfmt`, `clippy` and the `wasm32-unknown-unknown` target; rustup installs them on first use.

```sh
git clone https://github.com/Stefan-Paun/stapelwerk.git
cd stapelwerk
npm start              # build, then serve http://127.0.0.1:4860/stapelwerk/ (English: /stapelwerk/en/)
```

| Command | What it does |
|---|---|
| `npm run build` | release build of the WebAssembly core, copies `web/` to `dist/`, packs the levels, writes the English page and the service worker |
| `npm start` | build, then serve `dist/` like GitHub Pages |
| `npm test` | `cargo test --workspace`, then the Node tests in `test/unit`, `test/wasm` and `test/site` (`test/wasm` needs a build) |
| `npm run test:browser` | browser tests in headless Chrome over the DevTools protocol (set `CHROME_PATH` if Chrome is not found) |
| `npm run check` | everything CI runs, in the same order, with a summary: fmt, clippy, tests, build, differential test, level check, ISA table, no crates, no npm dependencies, WASM size, no private paths |

On a small machine, `export CARGO_BUILD_JOBS=2` keeps the Rust builds from using all cores. Locally the service worker is off; add `?sw=1` to the URL to test offline mode.

The command-line tool (binary `stapelwerk`, run it with `cargo run -q -p stapelwerk-cli -- <command>`):

| Command | Purpose |
|---|---|
| `asm <file.s> [-o out.img]` | assemble; print diagnostics or write a memory image |
| `dis <file.img\|file.s>` | disassemble |
| `run <file.s> [--in "3 4"] [--in-text "…"] [--trace]` | run a program in test mode |
| `run <file.s> --level <file.lvl> [--test NAME\|INDEX] [--trace]` | run a program against a level's tests exactly like the game |
| `check-levels [--strict] [--mutate] <dir\|file>` | check level files |
| `pack-levels <dir> <target-dir>` | pack levels for the web build |
| `isa-table` | print the generated instruction table |
| `fuzz-gen --seed S --count N --out <dir>` | generate programs for the native-vs-WebAssembly differential test |

Details and exit codes: [docs/LEVELS.md](docs/LEVELS.md#command-line-tool).

## Project structure

```
crates/core/   the machine: ISA table (isa.rs, the single source), CPU, memory, heap,
               system calls, assembler, disassembler, session with time travel,
               level parser and packer, test runner
crates/wasm/   the WebAssembly C ABI (the only crate with unsafe code)
crates/cli/    the stapelwerk command-line tool
web/           the site: index.html, css/, js/ (vm.js is the only WebAssembly module),
               js/texts/de.js and en.js (all interface texts), fonts/
levels/        30 .lvl files and manifest.txt (playing order and chapters)
docs/          the specification: ISA, ABI, LEVELS, ERRORS, CURRICULUM, TO-REAL-CPUS
tests/         shared test data: assembler golden files, ALU vectors
test/          Node tests (unit, wasm, site) and browser tests
tools/         build, serve, test, check and differential test scripts (plain Node)
```

## Add a level

Read [docs/CURRICULUM.md](docs/CURRICULUM.md) (chapters, tone, lengths, hint levels) and [docs/LEVELS.md](docs/LEVELS.md) (file format) first. Level 2.2 "Until Zero" is the template.

1. **Create the file.** Copy `levels/2-2-bis-zur-null.lvl` to `levels/<id>.lvl`. The id is `<chapter>-<order>-<german-slug>`, for example `3-5-zweierpotenz`, and must equal the `id` key. Set `chapter`, `order`, `kind` (`normal`, `tutorial`, `debug`, `final` or `bonus`), `budget`, `seed` and `mobile`.
2. **Write both languages completely.** `title.de`/`title.en`, `card.de`/`card.en` (one sentence), `[text.de]`/`[text.en]` (the task, at most 80 words plus an example), `[learned.de]`/`[learned.en]` ("What you just learned": one summary sentence, how it looks in C, how x86, ARM and RISC-V do it). German texts use "du"; no exclamation-mark chains. Statements about real processors must be correct; check them against [docs/TO-REAL-CPUS.md](docs/TO-REAL-CPUS.md), and when in doubt, leave them out.
3. **Write exactly three hints per language** in `[hint.de]` and `[hint.en]`, separated by a line `---`: hint 1 gives the idea, hint 2 names the tools, hint 3 is almost the solution (a scaffold with `; ???` gaps).
4. **Write the start code and the reference solution.** `[start]` is what the player sees; `[reference]` is a correct solution. For a `debug` level the start code contains the bug. Lower-case instructions, aligned columns, German labels without umlauts in start code.
5. **Write the tests.** 3–4 visible tests that cover the edge cases, plus 8–20 hidden ones (`hidden = yes`). Use `out`, `out_text`, `expect`, `expect_reg`, `expect_screen = reference` or `expect_heap_empty` as the level needs; function tests use `call = LABEL`.
6. **Add it to the manifest.** Put the file name into `levels/manifest.txt` under the right chapter line, in playing order.
7. **Check it.**

   ```sh
   cargo run -p stapelwerk-cli -- check-levels --strict --mutate levels/
   ```

   This verifies that the file parses, both languages are complete, the reference passes every test within 50 % of the budget, the start code and the empty program do not pass (except tutorials), and that deleting any single instruction of the reference makes some test fail. If a deletion survives, add a test; only a pure optimisation may carry `; mutate:skip` (at most two per level).
8. **Try your own solution and the bug path.**

   ```sh
   cargo run -q -p stapelwerk-cli -- run my-solution.s --level levels/<id>.lvl
   cargo run -q -p stapelwerk-cli -- run my-solution.s --level levels/<id>.lvl --test <name> --trace
   ```

9. **Play it in the browser.** `npm start`, then open `http://127.0.0.1:4860/stapelwerk/#/level/<id>` and the English page `/stapelwerk/en/#/level/<id>`. Read the texts once in both languages.
10. Run `npm run check` and open the pull request.

Not a developer? Open a "Level idea" issue with the learning goal, the task and the tests; someone can turn it into a file.

## Rules

- **No third-party crates**, also not as dev-dependencies. `Cargo.lock` must not contain any `source = ` line; CI checks this.
- **No npm packages**, no bundler, no CDN. Tools and tests are plain Node, the site is plain browser JavaScript.
- **Every interface text in German and English.** Texts live in `web/js/texts/de.js` and `web/js/texts/en.js` with the same keys and placeholders; a test checks that they match. Level texts need both `.de` and `.en` sections.
- **No external requests.** No fonts, scripts, images or analytics from other servers. The Content-Security-Policy in `web/index.html` allows only the site's own files; keep it that way.
- **Safe DOM.** No inline scripts or styles, no `innerHTML`: build nodes and set text with `textContent`/`setAttribute`. Level texts are a safe Markdown subset rendered into DOM nodes.
- **The machine stays deterministic.** No floats, no clocks, no `HashMap`/`HashSet` in the core, and no panic from any player input. Code comments are in English.
- **The specification changes only with a reason.** `docs/ISA.md`, `docs/ABI.md`, `docs/abi-exports.txt`, `docs/LEVELS.md` and `docs/ERRORS.md` are the contract between the core, the WebAssembly bridge, the interface and the levels. A change needs an explanation in the pull request and has to update the code and tests on every side. After changing `crates/core/src/isa.rs`, regenerate the table: `cargo run -q -p stapelwerk-cli -- isa-table > docs/isa-table.md`.
- **Public repository.** No private paths, e-mail addresses, keys or screenshots with personal data. `npm run check` looks for paths and e-mail-like text.

## Commits and pull requests

- Keep a pull request about one thing. Fill in the checklist of the pull request template.
- `npm run check` (or at least `npm test`) must pass. Check the exit code, not only the output.
- Write a short commit subject that says what changes, in English or German.
- For a change to the interface, add a screenshot in light and dark theme if it changes how something looks.

## Code of conduct

Be kind and assume good intent. Harassment or discrimination is not tolerated; the maintainer may remove comments or block people who do not follow this.
