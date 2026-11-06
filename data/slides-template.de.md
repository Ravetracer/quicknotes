---
presentation: true
title: Meine Präsentation
theme: default
transition: fade 400ms
paginate: true
footer: Meine Präsentation · QuickNotes
duration: 20m
---

<!-- class: lead -->

# Meine Präsentation

In Markdown geschrieben – Folien werden durch `---` getrennt

???
Notizen für den Vortragenden stehen unter drei Fragezeichen.
Sie sind nur in der Referentenansicht zu sehen (**Strg+F5**).

---

<!-- transition: slide-left 600ms -->

## Schritt für Schritt

::: steps fade-up dim
- Der erste Punkt erscheint per Klick oder Tastendruck
- dann der zweite
- und der dritte – frühere Punkte werden abgeblendet
:::

???
`::: steps` blendet Listenpunkte (oder Blöcke) nacheinander ein.
Animationen zum Ausprobieren: fade, fade-up, zoom, flip, bounce, drop, wipe, …

---

## Spalten und Positionen

::: columns
::: step zoom 0.8s
### Links
Beliebiges Markdown: Text, Listen, Code, Bilder
:::
|||
::: step fade-left
### Rechts
`|||` beginnt die nächste Spalte
:::
:::

::: place bottom right
_Unten rechts platziert_
:::

---

<!-- transition: zoom; background: #1e3a5f; color: white; class: lead -->

# Vielen Dank!

Bilder: `![w=400](foto.jpg)` · Hintergrund: `![bg](foto.jpg)`
