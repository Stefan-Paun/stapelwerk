# Security

Stapelwerk is a static site: HTML, CSS, JavaScript modules and one WebAssembly module, served from GitHub Pages. It has no server code, no accounts and no backend. The page sends a strict Content-Security-Policy (`default-src 'none'`, scripts and connections only from its own origin, no inline scripts or styles), builds the DOM only through `textContent`/`setAttribute`, and renders level texts from a small Markdown subset into DOM nodes.

## Supported versions

| Version | Supported |
|---|---|
| 1.0.x | yes |
| older | no |

The live site at <https://stefanpaun.page/stapelwerk/> always runs the latest version from `main`.

## Reporting a vulnerability

Please do **not** open a public issue. Use GitHub's private vulnerability reporting instead: **Security → Report a vulnerability** on this repository ([direct link](https://github.com/Stefan-Paun/stapelwerk/security/advisories/new)). This is the only reporting channel.

Please include what you did, what happened, and if possible a level file, program or share link that shows it. You will get an answer within a week. This is a hobby project, so there is no bug bounty.

## Scope

In scope:

- **The site:** script injection (XSS) through level contents, program code, output of a program, imported progress files or shared links (`#/sandbox?code=…`), and ways around the Content-Security-Policy.
- **The WebAssembly boundary** (`crates/wasm`, `web/js/vm.js`): memory errors, out-of-bounds reads or writes, crashes or hangs of the page caused by any input (programs, level files, arguments passed through the ABI).
- **The service worker** (`web/sw.template.js`): serving content from outside the site or caching it in a way that lets one page affect another.
- **CI and the build** (`.github/workflows/`, `tools/`): anything that lets a pull request run code with write access or leak secrets.

Out of scope:

- Reference solutions and hidden tests being readable in this repository and in the published level files. This is intentional; Stapelwerk is a practice tool, not an exam tool.
- Editing your own progress in local storage to unlock levels or badges.
- A program that runs slowly within its cycle budget, or the sandbox running a program you started yourself.
- Reports about GitHub Pages itself (please report those to GitHub).
