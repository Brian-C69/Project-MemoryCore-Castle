# The start-up screen

Castle shows a start-up screen while it loads: a picture, a funny line, a bar and what is loading.

- **The picture** is `loader.webp` in this folder: 16:9, for example 1920x1080, with the subject kept
  in the top and middle (the bottom third is covered by the words). Without it the screen shows its
  own night-sky gradient. Save it as a real WebP; a PNG renamed to `.webp` will not work.
- **More funny lines** go in `quips.json` in this folder: an array of strings, up to 60, each up to
  120 characters. They are added to the built-in ones.

Both are bundled into the build; nothing else needs changing.

A theme can override all of this for itself with a `loader` block in its `theme.json` (its own
picture, title, funny lines, colour and where the words sit); see MAKE-IT-YOURS.md, "A start-up
screen for your theme".
