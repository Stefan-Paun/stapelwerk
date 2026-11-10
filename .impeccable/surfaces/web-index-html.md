---
version: 1
slug: "web-index-html"
primary_target: "web/index.html"
related_targets: ["web/css/app.css","web/js/views/workspace.js"]
---

# Surface brief: web/index.html (Level-Arbeitsfläche `#/level/<id>`, dazu Levelwahl, Sandbox, Handbuch, Über)

## Scope und Modus

Operate. Die Arbeitsfläche ist ein Werkzeug: Aufgabe lesen, Code schreiben, assemblieren, testen, im Debugger Schritt für Schritt (auch rückwärts) nachsehen, was die Maschine tut. Levelwahl, Sandbox, Handbuch und Über erben dieselbe Welt; das Handbuch ist eine Read-Fläche innerhalb dieser Welt.

## Publikum, Aufgabe, Inhalt

Schüler ab 16 am Schullaptop (1366×768, 1280×720, oft unter Neonlicht im Informatiksaal), Studienanfänger abends am eigenen Laptop, Lehrkräfte am Beamer. Häufigste Handlung: Schritt / Zurück / Lauf, Register und Stack lesen, Tests starten. Wichtige Zustände: nicht assembliert, Assemblerfehler (Zeile/Spalte), pausiert, läuft, Haltepunkt, wartet auf Eingabe, Laufzeitfehler (fault.*), Budget erschöpft, Tests bestanden/gescheitert, Level gelöst (Kennzahlen, Abzeichen Kompakt/Flink). Inhalt: 30 Levels in 8 Kapiteln (Titel aus dem Lehrplan), Register r0–r7/fp/sp/pc, Flags Z/N/C/V, Stack-Rahmen, 64×64-Bildschirm mit 16 benannten Farben, Ausgabeband, Hexdump mit Segmenten, Heap-Blöcke.

## Entscheidungen ohne Rückfrage (Begründung)

- Keine Interview- oder Entscheidungsseite: der Auftrag verbietet Rückfragen an Menschen und Netzwerkdienste; Richtung und Schriften sind im Bauplan gepinnt. Bauweg code-led (kein Bildgenerator verfügbar); der Ehrgeiz steht im Vertrag unten.
- Hell als Standard, weil der Nutzungsort ein heller Saal oder Schreibtisch am Tag ist und Beamer helle Flächen besser zeigen; „Blaupause“ als vollwertiges dunkles Gegenstück für den Abend, per System oder Schalter.
- Zwei Schriften, beide gepinnt: Sofia Sans (variable, Gewichte 400–700) für Bedienung und Text, IBM Plex Mono (400/500/600, kursiv) nur für Code, Adressen, Werte und Messzahlen, nie als Technik-Kostüm.
- Farbstrategie „Restrained mit Rollen“: Blattgrund plus Tusche, Zeichenblau nur für Handlung, aktuelle Zeile und Auswahl, Rotstift nur für Fehler und Überschriebenes, Stempelgrün nur für Bestanden. Die Kopfleiste trägt Zeichenblau als Feld, damit die Welt nicht nur eine Akzentfarbe ist.

## Gewürfelte Runde (Impeccable, Seed f82b416c, Index 5)

Eigene Liste nach Resonanz (Welt der Zielgruppe: Schule, erstes Semester, Technik-Unterricht):
1 Reißbrett / technische Zeichnung nach ISO 128 (Bauplan-Pin), 2 kariertes Schulheft mit Füller und Rotstift, 3 Tafelbild im Hörsaal, 4 Netzplan der Wiener U-Bahn, **5 Codierbogen (Programmierformular der Lochkartenzeit) – zugewiesen**, 6 Schaltplan und Platinenlayout, 7 Normblatt/Formelsammlung.
Der Bauplan pinnt „Reißbrett“; ein gepinnter Entscheid schlägt den Wurf. Der zugewiesene Codierbogen bleibt als Spender: Raise „Spaltendisziplin“: Code steht in festen Spalten (Label | Befehl | Operanden | Kommentar), der Editor zeichnet die Spaltenlinien als Haarlinien, die Startcodes sind danach ausgerichtet.
Herausforderer (fusioniert mit Stapelwerk, gewogen nach Wiedererkennung und Klarheit):
- Variable-Font-Specimen: abgelehnt; Raise „Maßstab statt Schmuck“: Hierarchie nur über Größe und Gewicht auf einer festen Skala 1.2, Grundlinienraster 4 px.
- Sternatlas: abgelehnt (zu nah an Resonanz); Raise „feste Größenrampe“: Wichtigkeit in Strichstärke auf einer festen Rampe 1 / 1,5 / 2 / 3 px statt in Kästen.
- Mittelschienen-Satz mit Linierwerk: konkurrenzfähig bei Klarheit, verliert Wiedererkennung; Raise „Linierwerk ist echt“: Maßketten am Stack werden aus den echten Zeilenpositionen gemessen und bei jeder Größenänderung neu gezogen; Zustände als gedruckte Marken (Haarlinie in Ruhe, gefüllt gedrückt, geklammert im Fokus).
- Warme Consumer-App: abgelehnt; Raise „Farbe heißt handeln“: Zeichenblau erscheint nur, wo man etwas tun kann oder wo die Maschine gerade steht.
- Bordkarte und Abflugtafel: abgelehnt; Raise „Änderung bleibt markiert“: was der letzte Schritt verändert hat, bleibt schraffiert und rot unterstrichen, bis der nächste Schritt kommt.
- Saville-Katalog: abgelehnt; Raise „Farbcode mit Legende“: Speichersegmente tragen einen Farbcode, aber immer mit Namen daneben.

## Direction contract

THESIS: Die Maschine als technische Zeichnung, die man bedienen kann: Register, Stack und Speicher sind bemaßte Bauteile auf einem Zeichenblatt, der Debugger zeichnet jede Änderung nach. Verweigert: das Hacker-Terminal (grün auf schwarz) und die IDE-Kopie mit grauen Kästen und Schatten.

OWN-WORLD: Blatt #F3F5F7, Tusche #16222E, Zeichenblau #1D4F91 (Kopfleiste als Feld, Handlung, aktuelle Zeile), Rotstift #C6372F (Fehler, Überschriebenes), Stempelgrün #2F7D55 (bestanden). Blaupause dunkel: Grund #0F2A4A, Linien und Schrift hellblau-weiß. Sofia Sans für Bedienung, IBM Plex Mono für Code und Werte, tabellarische Ziffern. Tafeln sind nicht umrahmt: Strichstärken 1/1,5/2/3 px trennen, keine Schatten, keine Rundungen über 2 px. Geänderte Bytes und Register schraffiert (45°), Maßketten mit Schrägstrichen als Endmarken.

STORY: Wer ein Level öffnet, sieht links Aufgabe und Code, in der Mitte Register und Stack, rechts Bildschirm und Ausgabe, und weiß sofort, wo „Schritt“ ist. Jeder Schritt markiert, was er verändert hat; der Stack zeigt bemaßt, wie weit fp vom Rücksprung entfernt ist. Gelöst wird mit Stempel und Kennzahlen gegen die Referenz.

FIRST VIEWPORT: 1440×900: Kopfleiste Zeichenblau 48 px (Wortmarke, Kapitel und Level-Titel, Navigation, Sprache, Thema). Darunter Debugger-Leiste 52 px: „Assemblieren & testen“ (Zeichenblau gefüllt, die einzige gefüllte Handlung), Neustart, Zurück, Schritt, Lauf/Pause, Zeitleiste über die restliche Breite mit Schrittzähler und Status. Drei Spalten 38 / 25 / 37 %: links Reiter Aufgabe/Hinweise/Tests über dem Editor (Zeilenspalte, Haltepunkte, aktuelle Zeile, Zeilenzähler, Spaltenlinien), Mitte Register als Definitionsliste mit Flags, darunter Stack-Tabelle mit Maßketten links, rechts Bildschirm 64×64 (Soll/Ist), Konsole, Reiter Hexdump/Heap/Handbuch. Primäre Handlung: Schritt und Assemblieren & testen.

FORM: Reißbrett (technische Zeichnung), vom Bauplan gepinnt, Platz 1 der eigenen Liste; Wurf f82b416c wies Platz 5 (Codierbogen) zu, der als Spaltendisziplin eingeht. Signatur: Maßketten am Stack, live aus dem Layout gemessen. Motion: 150–200 ms Zustandswechsel, Schraffur blendet beim Schritt ein, reduced motion schaltet alles hart.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Offen

- `vm.js` (echter WASM-Kern) folgt; bis dahin `vm-mock.js` mit demselben Session-Vertrag.
- `og.png` und Demo-Bilder erst mit echtem Kern.
- DESIGN.md schreibt der Documenter nach dem Finish-Review.
