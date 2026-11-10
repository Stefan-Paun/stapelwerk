---
name: Stapelwerk
description: Assembly puzzles on a home-made 32-bit CPU, drawn as an operable technical drawing.
colors:
  paper: "#F3F5F7"
  paper-2: "#E6EBF0"
  paper-3: "#FFFFFF"
  ink: "#16222E"
  ink-2: "#435160"
  ink-3: "#5E6B78"
  rule: "#16222E"
  rule-mid: "#8E9AA6"
  rule-faint: "#D3DAE1"
  blue: "#1D4F91"
  blue-strong: "#143B6E"
  blue-wash: "#DCE6F3"
  blue-on: "#FFFFFF"
  pencil: "#B9322A"
  pencil-wash: "#F7E1DE"
  stamp: "#2A714D"
  stamp-wash: "#DDEFE4"
  ochre: "#8A5B00"
  ochre-wash: "#F6ECD5"
  head-bg: "#1D4F91"
  head-ink: "#F3F5F7"
  head-ink-2: "#C9D8EC"
  head-rule: "#143B6E"
  seg-text: "#1D4F91"
  seg-data: "#2F7D55"
  seg-input: "#8A5B00"
  seg-heap: "#7A3C8F"
  seg-stack: "#0E7489"
  seg-screen: "#16222E"
  bp-paper: "#0F2A4A"
  bp-paper-2: "#0B2240"
  bp-paper-3: "#133259"
  bp-ink: "#EAF2FB"
  bp-ink-2: "#B9CDE4"
  bp-ink-3: "#93ADCB"
  bp-rule: "#D6E4F5"
  bp-rule-mid: "#557AA4"
  bp-rule-faint: "#24486F"
  bp-blue: "#9CCBFF"
  bp-blue-strong: "#C6E1FF"
  bp-blue-wash: "#1C4675"
  bp-blue-on: "#0B2240"
  bp-pencil: "#FF958B"
  bp-pencil-wash: "#4A2B45"
  bp-stamp: "#84DDAE"
  bp-stamp-wash: "#174640"
  bp-ochre: "#F2C46B"
  bp-ochre-wash: "#3E3A35"
  bp-head-bg: "#0A1D35"
  bp-head-ink-2: "#A9C2DE"
  bp-head-rule: "#2A5383"
typography:
  display:
    fontFamily: "Sofia Sans, Segoe UI, system-ui, sans-serif"
    fontSize: "2.25rem"
    fontWeight: 750
    lineHeight: 1.2
    letterSpacing: "-0.015em"
  headline:
    fontFamily: "Sofia Sans, Segoe UI, system-ui, sans-serif"
    fontSize: "1.75rem"
    fontWeight: 650
    lineHeight: 1.2
    letterSpacing: "-0.005em"
  title:
    fontFamily: "Sofia Sans, Segoe UI, system-ui, sans-serif"
    fontSize: "1.375rem"
    fontWeight: 650
    lineHeight: 1.2
    letterSpacing: "-0.005em"
  pane-title:
    fontFamily: "Sofia Sans, Segoe UI, system-ui, sans-serif"
    fontSize: "0.9375rem"
    fontWeight: 700
    lineHeight: 1.2
    letterSpacing: "0.005em"
  body:
    fontFamily: "Sofia Sans, Segoe UI, system-ui, sans-serif"
    fontSize: "0.9375rem"
    fontWeight: 400
    lineHeight: 1.45
  lede:
    fontFamily: "Sofia Sans, Segoe UI, system-ui, sans-serif"
    fontSize: "1.125rem"
    fontWeight: 400
    lineHeight: 1.45
  label:
    fontFamily: "Sofia Sans, Segoe UI, system-ui, sans-serif"
    fontSize: "0.8125rem"
    fontWeight: 600
    lineHeight: 1.45
  caption:
    fontFamily: "Sofia Sans, Segoe UI, system-ui, sans-serif"
    fontSize: "0.75rem"
    fontWeight: 650
    lineHeight: 1.45
  code:
    fontFamily: "IBM Plex Mono, ui-monospace, SF Mono, Menlo, Consolas, monospace"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: "1.375rem"
    letterSpacing: "0"
    fontFeature: "\"tnum\", \"zero\""
  value:
    fontFamily: "IBM Plex Mono, ui-monospace, SF Mono, Menlo, Consolas, monospace"
    fontSize: "0.8125rem"
    fontWeight: 500
    lineHeight: 1.45
    fontFeature: "\"tnum\", \"zero\""
rounded:
  pip: "1px"
  sheet: "2px"
  balloon: "50%"
spacing:
  s1: "4px"
  s2: "8px"
  s3: "12px"
  s4: "16px"
  s5: "24px"
  s6: "32px"
  s7: "48px"
  s8: "64px"
components:
  button:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    typography: "{typography.body}"
    rounded: "{rounded.sheet}"
    padding: "0 12px"
    height: "36px"
  button-hover:
    backgroundColor: "{colors.paper-3}"
    textColor: "{colors.ink}"
  button-active:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.paper}"
  button-primary:
    backgroundColor: "{colors.blue}"
    textColor: "{colors.blue-on}"
    rounded: "{rounded.sheet}"
    padding: "0 12px"
    height: "36px"
  button-primary-hover:
    backgroundColor: "{colors.blue-strong}"
    textColor: "{colors.blue-on}"
  button-quiet:
    backgroundColor: "transparent"
    textColor: "{colors.blue}"
    rounded: "{rounded.sheet}"
    padding: "0 12px"
    height: "36px"
  button-small:
    typography: "{typography.label}"
    padding: "0 8px"
    height: "28px"
  button-danger:
    backgroundColor: "{colors.pencil}"
    textColor: "{colors.paper-3}"
    rounded: "{rounded.sheet}"
  icon-button:
    backgroundColor: "transparent"
    textColor: "{colors.ink-2}"
    rounded: "{rounded.sheet}"
    size: "36px"
  chip:
    backgroundColor: "transparent"
    textColor: "{colors.ink-2}"
    typography: "{typography.label}"
    rounded: "{rounded.sheet}"
    padding: "0 8px"
    height: "28px"
  chip-pressed:
    backgroundColor: "{colors.paper-3}"
    textColor: "{colors.ink}"
  segmented-pressed:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.paper}"
    height: "26px"
  flag-on:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.paper}"
    rounded: "{rounded.sheet}"
    height: "26px"
  input:
    backgroundColor: "{colors.paper-3}"
    textColor: "{colors.ink}"
    rounded: "{rounded.sheet}"
    padding: "0 8px"
    height: "32px"
  appbar:
    backgroundColor: "{colors.head-bg}"
    textColor: "{colors.head-ink}"
    height: "48px"
    padding: "0 16px"
  debugger-bar:
    backgroundColor: "{colors.paper-2}"
    textColor: "{colors.ink}"
    padding: "8px 16px"
  tab:
    backgroundColor: "transparent"
    textColor: "{colors.ink-2}"
    typography: "{typography.label}"
    padding: "0 8px"
    height: "36px"
  tab-selected:
    textColor: "{colors.ink}"
---

# Design System: Stapelwerk

## Overview

**Creative North Star: "The Operable Drawing"**

The machine is a technical drawing you can operate. Registers, stack and memory are dimensioned parts on a drawing sheet; the debugger redraws every change. Two themes are equal citizens: "Reißbrett" (drawing board, light, the default) and "Blaupause" (blueprint, dark, by system preference or toggle). Both carry identical roles; only values change.

Density is that of a workbench: a 48 px blue app bar, a debugger bar, then three columns (38 / 25 / 37) with no outer frames. Panels are separated by line weight, not by boxes, cards or shadows. Expression lives in drafting details that do real work: dimension chains measured from the live stack layout, 45° hatching on whatever the last step changed, a timeline drawn as a ruler, coding-sheet column rules in the editor, circled chapter balloons, an approval stamp on a solved level.

Rejected by the build and the product brief: the hacker terminal (green on black), the IDE clone of grey boxes and drop shadows, pixel-art costume.

**Key Characteristics:**
- Sheet and ink neutrals; colour only where it has a job.
- Hierarchy through stroke weight on a fixed ramp (1 / 1.5 / 2 / 3 px), never through containers.
- Sofia Sans operates the interface; IBM Plex Mono carries code, addresses, values and counts, always with tabular, slashed-zero figures.
- Changed state is marked with colour plus pattern (hatch, underline), never colour alone.
- Corners stay at 2 px; circles appear only as drafting balloons, nodes and status dots.

## Colors

Restrained with roles: a cool sheet and dark ink, one drafting blue, and three signal colours that each own exactly one meaning. Every text token was checked for AA against its ground in both themes.

### Primary
- **Drafting Blue** (`blue`): action and the machine's current position. The single filled action (Assemble & test), the current editor line and pc arrow, selected-tab underline, focus ring, timeline thumb, sp/fp labels, the next station on the chapter plan. In Blaupause it inverts to a light blue with dark text on fills (`bp-blue`, `bp-blue-on`).
- **Blue Field** (`head-bg`): the app bar is a full field of drafting blue in the light theme, so the world is more than an accent; in Blaupause it deepens to a near-black navy (`bp-head-bg`).

### Secondary
- **Red Pencil** (`pencil`, wash `pencil-wash`): errors and overwritten values only. Assembler error lines, wrong output (struck through), changed registers/stack slots/bytes (with hatch), breakpoint marks, fault status, destructive buttons.
- **Stamp Green** (`stamp`, wash `stamp-wash`): passed only. Solved stations, passed tests, earned badges, the "Bestanden" stamp, reached-reference deltas.

### Tertiary
- **Ochre** (`ochre`, wash `ochre-wash`): warning and heat. Unassembled edits ("dirty"), the hottest line in the line counter, metrics over the reference.

### Neutral
- **Drawing Sheet** (`paper`): page and pane ground. `paper-2` is the debugger bar and inline code; `paper-3` is the raised sheet (editor, inputs, hover, dialogs).
- **Ink** (`ink`): text and the heavy rules; `ink-2` secondary text, `ink-3` tertiary text, addresses, hex digits, axis ticks.
- **Rules** (`rule` major divisions, `rule-mid` column dividers and control strokes, `rule-faint` row hairlines).

### Memory segment code
`seg-text`, `seg-data`, `seg-input`, `seg-heap`, `seg-stack`, `seg-screen` colour the hexdump segment chips and sp marker. They always appear with the segment name beside them.

### Screen palette
The 16 screen colours live only in `web/js/palette.js`. They were tuned in OKLCH for separation under normal vision and three simulated colour-vision deficiencies, are identical in both themes (pixels are data), and always sit on index 0 (BLACK). They are not UI tokens and never style chrome.

### Named Rules
**The Colour-Has-a-Job Rule.** Blue means "you can act here" or "the machine is here"; red pencil means error or overwritten; stamp green means passed; ochre means warning. A colour never appears decoratively or outside its job.

**The One Filled Action Rule.** Per view, one action is filled blue. Everything else is a 1 px outline, quiet text, or icon button.

**The Legend Rule.** Coded colour (segments, heap states, screen pixels) always ships with a name, legend or inspector readout.

## Typography

**Display Font:** Sofia Sans (variable, self-hosted; fallback Segoe UI, system-ui)
**Label/Mono Font:** IBM Plex Mono 400/500/600 + italic (self-hosted; fallback ui-monospace, SF Mono, Menlo)

**Character:** A sober, slightly condensed grotesque for operating the tool, paired with a plain engineering mono that only ever holds machine facts. Mono is never a tech costume for headings or labels.

### Hierarchy
- **Display** (750, 36 px, 1.2, -0.015em): page titles (chapter plan, About, 404). Drops to 28 px on phones.
- **Headline** (650, 28 px): manual article title, manual TOC title.
- **Title** (650, 22 px): chapter names, result heading, dialog titles, prose h2.
- **Pane title** (700, 15 px): pane heads (Register, Stack, Bildschirm, Programm). Workspace level title is 700 at 18 px.
- **Body** (400, 15 px, 1.45; task text 1.5): task text, prose up to 70ch, lede at 18 px up to 62ch.
- **Label** (600, 13 px): buttons in small size, tabs, chips, tests, status, metrics.
- **Caption** (650, 12 px): table heads, register group heads, pane notes, legends. Sentence case, no tracking.
- **Code** (mono 400, 14 px / 22 px; 13 / 20 px on short laptop screens): editor and gutter, tab size 8, ligatures off.
- **Value** (mono 500, 13 px): register, stack, hexdump (12 px) and step-counter values.

### Named Rules
**The Scale-Not-Ornament Rule.** Hierarchy comes only from size and weight on the fixed ramp (12 / 13 / 15 / 18 / 22 / 28 / 36). No colour, tracking or caps to signal importance.

**The Mono-Means-Machine Rule.** IBM Plex Mono only for code, addresses, register and memory values, counts and identifiers, always `tabular-nums slashed-zero`.

## Layout

A 4 px grid (`s1`–`s8`: 4, 8, 12, 16, 24, 32, 48, 64). The workspace fills the viewport below the app bar with no page scroll: head row (level title, breadcrumbs, status), debugger bar, optional notice, then a three-column grid at 38 / 25 / 37 fractions. Column A stacks task tabs over the editor (the result tab takes 62 % of the column when shown); column B registers over stack; column C screen, console, then memory tabs.

Content pages (chapter plan, manual, about) sit in a centred column (1200 px; plan 1240 px; about 820 px) with 32 / 24 / 64 px padding. The manual is a 220 px sticky TOC plus article.

Responsive behaviour, all observed:
- **≤1599 px:** shortcut hints in debugger buttons and the cycle readout hide.
- **≥1200 px and ≤820 px tall** (1366×768, 1280×720): code drops to 13/20 px, rows and pane heads tighten, screen shrinks.
- **≤1199 px:** two columns; A spans full height, B over C on the right. Debugger buttons become icon-only; the chapter plan shows three stations per row.
- **≤759 px:** one column. The workspace becomes tabbed panes with a sticky tab strip; the debugger bar docks to the bottom with stacked icon-plus-label buttons; the editor gains a chip palette; the app bar wraps nav onto a second row (hidden inside the workspace); chapter stations run vertically.
- **Coarse pointer:** every control reaches 44 px.

## Elevation & Depth

Flat. Depth is conveyed by stroke weight and tonal sheets: `paper-2` for the debugger bar, `paper-3` for raised surfaces (editor, inputs, dialogs). `box-shadow` in the build is used only as drawn lines: inset 2–3 px underlines and left bars for current, waiting, invalid and changed states, a 1 px rule under sticky table heads, and the double ring around the next station.

### Named Rules
**The Line-Weight Rule.** Separation uses the stroke ramp: 2 px ink for major divisions (below the debugger bar, plan head, manual title), 1.5 px ink between stacked panes, frame heads and above result/learned blocks, 1 px mid between columns and around controls, 1 px faint for rows; 3 px is reserved for selection underlines. No borders around panels.

## Shapes

Square drafting geometry. Rectangles take a 2 px corner (`sheet`) at most; switch knobs and the timeline thumb take 1 px. Circles (`balloon`) are reserved for drafting balloons and nodes: chapter numbers, station nodes, badge marks, status dots, pending-test rings. Locked or unavailable things switch their stroke to dashed instead of fading. Icons are authored SVG on one 24-unit grid, 1.6 stroke, square caps, mitred joins.

Signature drafting forms: dimension chains with oblique tick ends (stack frames, chapter-plan station line), a ruler track with tick marks (timeline), dashed coding-sheet column rules at columns 9 and 28 in the editor, 45° hatching for changed values, and a rotated double-ruled stamp.

## Components

### Buttons
Printed marks: hairline at rest, ink-filled when pressed.
- **Shape:** 2 px corners, 36 px tall (28 px small; 44 px on touch), 12 px side padding, label 600 at 15 px.
- **Default:** transparent with a 1 px `rule-mid` outline. Hover lifts to `paper-3` with an ink outline; active fills ink with paper text. Transitions 150 ms on colour only.
- **Primary:** filled drafting blue with `blue-on` text; hover `blue-strong`. One per view.
- **Quiet:** no outline, blue text; hover adds the mid outline.
- **Danger:** filled red pencil (confirm delete only); a quiet variant uses a 50 % pencil outline.
- **Disabled:** ink-3 text and a dashed outline.
- **Icon button:** 36 px square, borderless, ink-2; hover `paper-2`.
- **Toggles in the debugger bar** (Run/Pause) fill ink when `aria-pressed`.
- **Keyboard hints:** `kbd` caps with a 1 px outline and a 2 px bottom edge.

### Chips
- **Style:** 28 px, 1 px faint outline, ink-2 label 13 px; hover mid outline.
- **State:** pressed gets ink outline, `paper-3` ground, weight 600. Segment chips carry an 8 px colour square plus the name.
- **Segmented control:** 1 px mid outline around joined buttons; pressed segment fills ink.

### Inputs / Fields
- **Style:** 32 px, `paper-3` ground, 1 px mid outline, 2 px corners.
- **Focus:** 2 px drafting-blue outline (offset 0 on fields, 2 px elsewhere).
- **Waiting / Error:** blue or pencil outline plus a 2 px inset underline.
- **Switch:** 34×18 square track, 1.5 px stroke, square knob; checked fills blue.

### Navigation
- **App bar:** 48 px blue field, wordmark 750 at 18 px with the stack logo, nav links 550 in `head-ink-2`; current page goes `head-ink` with a 3 px underline. Language link and icon buttons on the right; focus ring switches to `head-ink`.
- **Tabs:** 13 px / 600 labels, selected tab turns ink and gets a 3 px blue underline. Same treatment for the phone pane tabs.
- **Manual TOC:** a 1 px mid rule on the left; the current entry gets a 2 px blue left rule and weight 650. On phones it becomes wrapped outline chips.

### Debugger bar
`paper-2` strip under a faint top rule and over a 2 px ink rule: primary Assemble & test, then Restart / Back / Step / Run, then the timeline ruler (1.5 px ink baseline, 10 % tick marks, a 4×20 px blue thumb) with the step readout in mono, then the test-case select.

### Register list and flags
Definition list, 26 px rows on faint hairlines: name 600, hex in ink-3, decimal value right-aligned. A value the last step changed gets the hatch, a red 2 px underline and the struck old value beside it; values touched earlier in a run keep a quiet 1 px ink-3 underline. Flags are 26 px outlined pills of 2 px corners; set flags fill ink.

### Stack table with dimension chains (signature)
A real table (address, value, meaning) with a 56 px SVG gutter on the left where dimension chains (1.25 stroke, oblique ticks, mono labels like `fp+4`) are measured from the rendered rows and redrawn on every layout change. Frame heads sit on a 1.5 px ink rule; the sp row closes with a 2 px blue rule; the current return chain is blue.

### Editor
Gutter on `paper` (line counter in blue, line number, breakpoint dot in pencil, blue pc arrow) beside a `paper-3` sheet. Current line: `blue-wash` with a 3 px blue left bar. Error line: `pencil-wash` with a wavy pencil underline on the token. Hot line: `ochre-wash` with an ochre bar. Errors list under a 1.5 px pencil rule.

### Notices
Full-width strips with a 1 px rule in the signal colour, signal wash ground, signal-coloured icon and ink text (error, warning, ok).

### Result
A rotated (-4°) stamp: 2 px stamp-green border plus a 1 px outline offset 2 px, 800 weight, uppercase, 0.12em tracking, 13 px. Below it a metrics table (1.5 px ink rule under the head, numbers 600 right-aligned, deltas in stamp or ochre) and badges as circle marks, dashed until earned.

### Chapter plan
Each chapter: a 40 px balloon with a 1.5 px ink stroke with the number (filled ink when complete, dashed when locked), title, then up to five stations on a 1.5 px ink dimension line with oblique ticks. Station nodes are 18 px circles with a 1.5 px stroke: green with a check when solved, blue double ring when next, dashed when locked. Station number in mono, title 650, card text 13 px, tags 12 px, earned badges as small green-outlined tags.

## Do's and Don'ts

### Do:
- **Do** separate panes and sections with the stroke ramp (1 / 1.5 / 2 / 3 px) and leave panels unframed.
- **Do** keep exactly one filled drafting-blue action per view.
- **Do** mark every change with colour plus pattern: hatch and underline for changed values, wavy underline for errors, struck-through for wrong output.
- **Do** set every value, address and count in IBM Plex Mono with tabular, slashed-zero figures.
- **Do** switch locked or unavailable things to dashed strokes rather than lowering opacity.
- **Do** define both themes for any new token; Blaupause is a full counterpart, not an afterthought.
- **Do** keep state transitions at 150 ms on `cubic-bezier(0.2, 0.7, 0.2, 1)` and drop them to 0 under reduced motion; supply forced-colours fallbacks for hatch, current line and selected tabs.

### Don't:
- **Don't** use green-on-black terminal styling, grey IDE boxes, or pixel-art chrome.
- **Don't** use drop shadows or cards for panels.
- **Don't** round rectangles beyond 2 px; circles are for balloons, nodes and dots only.
- **Don't** use red pencil, stamp green or ochre for anything outside error, passed and warning.
- **Don't** let the screen palette leak into UI chrome, or theme the screen pixels.
- **Don't** convey state by colour alone; always pair it with a name, mark or pattern.
