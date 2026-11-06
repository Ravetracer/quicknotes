---
presentation: true
title: My Presentation
theme: default
transition: fade 400ms
paginate: true
footer: My Presentation · QuickNotes
duration: 20m
---

<!-- class: lead -->

# My Presentation

Written in Markdown – slides are separated by `---`

???
Presenter notes go below three question marks.
They are only shown in the presenter view (**Ctrl+F5**).

---

<!-- transition: slide-left 600ms -->

## Step by step

::: steps fade-up dim
- The first point appears on click or key press
- then the second one
- and the third one – earlier points are dimmed
:::

???
`::: steps` reveals list items (or blocks) one by one.
Try the animations: fade, fade-up, zoom, flip, bounce, drop, wipe, …

---

## Columns and positions

::: columns
::: step zoom 0.8s
### Left
Any Markdown: text, lists, code, images
:::
|||
::: step fade-left
### Right
`|||` starts the next column
:::
:::

::: place bottom right
_Placed bottom right_
:::

---

<!-- transition: zoom; background: #1e3a5f; color: white; class: lead -->

# Thank you!

Images: `![w=400](photo.jpg)` · background: `![bg](photo.jpg)`
