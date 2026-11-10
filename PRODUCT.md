# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

delegated (vom Bauvertrag festgelegt): statische Seite ohne Bundler und ohne npm-Pakete. Oberfläche in Vanilla-JS (ES-Module), handgeschriebenes CSS, CPU-Kern in Rust ohne fremde Crates, als WebAssembly geladen (`web/js/vm.js` ist die einzige WASM-Stelle). Veröffentlicht über GitHub Pages unter `https://stefanpaun.page/stapelwerk/` (deutsch) und `/stapelwerk/en/` (englisch). Begründung: jeder kann den Code ohne Werkzeugkette lesen, forken und offline benutzen; Schulrechner brauchen keine Installation.

## Users

- Oberstufen-Schüler ab 16 (Informatik-Unterricht, Matura-Vorbereitung): spielen am Schul- oder eigenen Laptop, oft 1366×768 oder 1280×720, manchmal am Handy zwischendurch.
- Studienanfänger Informatik (erstes Semester Rechnerarchitektur/Technische Informatik): üben Stack-Rahmen, Aufrufkonvention, Zweierkomplement neben der Vorlesung, abends am eigenen Laptop.
- Lehrkräfte: zeigen Levels per Beamer, geben einzelne Levels als Übung auf, brauchen Datenschutz ohne Konto.
Aufgabe in jedem Fall: ein Assembler-Rätsel lösen, und dabei mit dem Debugger verstehen, was die Maschine Schritt für Schritt tut.

## Product Purpose

Stapelwerk ist ein Lernspiel im Browser: Man löst Assembler-Rätsel auf einer selbst entworfenen 32-Bit-Lehr-CPU. 30 Levels in 8 Kapiteln führen von „Register ändern“ bis zu Heap-Listen; jedes Level hat Aufgabe, drei Hinweise, sichtbare und verdeckte Tests, Referenzwerte und „Was du gerade gelernt hast“. Erfolg heißt: Lernende können danach einen Stack-Rahmen an die Tafel zeichnen, eine Aufrufkonvention erklären und einen Fehler mit dem Debugger statt durch Raten finden.

## Positioning

Stapelwerk schließt die Lücke zwischen den Modellrechnern der Schule (Johnny, Bonsai, Little Man Computer) und den Uni-Werkzeugen (MARS, RARS, Ripes): klein genug für einen Nachmittag, aber mit allem, was im ersten Semester abgeprüft wird (Zweierkomplement, Flags, Stack-Rahmen, Aufrufkonvention, Heap, speicherabgebildete Ein-/Ausgabe). Anders als kommerzielle Programmier-Rätselspiele lehrt jedes Level, was ein echter x86-, ARM- oder RISC-V-Prozessor und ein C-Compiler tun, und Fehlersuche ist eine eigene Spielmechanik (Debugger mit Zeitreise: Rückwärtsschritt, „rückwärts bis zur letzten Änderung“).

## Operating Context

- Ein Level wird in einer Sitzung von 10–40 Minuten gespielt: Aufgabe lesen, Code schreiben, assemblieren, Tests laufen lassen, im Debugger schrittweise prüfen, verbessern.
- Bildschirmgrößen: Schullaptops (1366×768, 1280×720), eigene Laptops (1440×900 und größer), Beamer, Tablet; Handy nur für ausgewählte Levels (`mobile = yes`).
- Kein Konto: Fortschritt nur lokal (localStorage mit Präfix `stapelwerk:`), Export/Import als JSON-Datei.
- Offline-fähig über Service Worker; keine Anfragen an fremde Server.
- Die Maschine: 16 Register-Codes (r0–r7, fp, sp), Flags Z/N/C/V, 64 KiB Speicher mit Segmenten (null, text, data, input, heap, guard, stack, screen), 64×64-Bildschirm mit 16 benannten Farben, Systemaufrufe für Ein-/Ausgabe und Heap.

## Capabilities and Constraints

- Debugger: Einzelschritt, Rückwärtsschritt, Lauf/Pause in Zeitscheiben, Neustart, Zeitleiste (goto), Haltepunkte, Register und Flags im Pausenzustand bearbeiten, „rückwärts bis zur letzten Änderung“, Zeilenzähler, Rahmenkette, Heap-Liste, Hexdump.
- Sprachen: Deutsch und Englisch vollständig, flache i18n-Schlüssel; Hash-Routen sprachneutral (`#/`, `#/level/<id>`, `#/manual/<topic>`, `#/sandbox`, `#/about`).
- Sicherheit: strenge CSP ohne Inline-Skripte/-Styles, DOM nur über `textContent`/`setAttribute`, Level-Texte als sichere Markdown-Teilmenge.
- Barrierefreiheit: Tastaturbedienung, Stack als echte Tabelle, Live-Region für Debugger-Schritte, Kontrast AA, `prefers-reduced-motion`, `forced-colors`, nie nur Farbe.
- Thema ist ausschließlich Rechnerarchitektur; keine Sicherheits- oder Angriffsthemen.
- Offen: Demo-GIF und Screenshots für das README entstehen erst nach dem echten Kern.

## Brand Commitments

- Name: **Stapelwerk** (Wortspiel aus Stapel = Stack und Werk/Werkstatt).
- Ton: per du, sachlich und warm; keine Ausrufezeichen-Ketten, keine Verkleinerungsformen, kein „super!“. Fachwörter beim ersten Auftreten fett mit Handbuch-Link; englische Uni-Fachbegriffe (Stack, Heap, Register, Flag) werden beim ersten Mal eingedeutscht.
- Bindende visuelle Vorgabe aus dem Bauplan: Richtung „Reißbrett“ (technische Zeichnung), hell mit dunklem Gegenstück „Blaupause“; Schriften Sofia Sans und IBM Plex Mono, selbst gehostet (OFL). Ausdrücklich nicht: Hacker-Grün, Terminal-Kostüm, Apple-Look von stefanpaun.page, Indigo/Bernstein von Resonanz, Pixel-Look.
- Ehrlichkeit: Aussagen über echte Prozessoren müssen stimmen; Grenzen (erfundene CPU, kein Pipeline-/Cache-Modell) werden offen genannt.

## Evidence on Hand

- Kapitelplan mit 30 Level-Titeln (DE/EN), Lernzielen und Musterlevel 2.2 „Bis zur Null“ im Lehrplan.
- Noch nicht vorhanden: der echte Rust-Kern als WASM (die Oberfläche läuft bis dahin auf einem JS-Mock mit Beispieldaten), Nutzerzahlen, Erfahrungsberichte aus dem Unterricht. Nichts davon darf erfunden werden.

## Product Principles

1. Die Maschine ist sichtbar: jeder Schritt zeigt, was sich geändert hat und warum.
2. Fehler sind Lernstoff: Fehlermeldungen nennen Ursache, Ort und nächsten Schritt; Fehlersuche ist Spielmechanik.
3. Echt statt Spielzeug: was ein Level lehrt, stimmt auch auf x86, ARM und RISC-V.
4. Keine Hürde: kein Konto, keine Daten, offline, läuft auf dem Schullaptop.
5. Ruhige Werkbank: die Oberfläche tritt hinter die Aufgabe zurück; Ausdruck lebt in präzisen Details.

## Accessibility & Inclusion

WCAG 2.2 AA als Ziel: Tastatur-only spielbar, sichtbarer Fokus, Tippflächen ≥ 44 px, Live-Region, Kontrast nachgerechnet, farbenblind-taugliche Bildschirmpalette, geänderte Werte immer mit Farbe plus Zeichen/Muster.
