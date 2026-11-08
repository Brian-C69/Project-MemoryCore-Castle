# Hair Turnaround

**Purpose:** a head-and-shoulders sheet of one hairstyle - front, side and back views in one image on a plain dark background - so the hairstyle can be drawn the same way every time. Once it exists, this sheet is the hairstyle reference.

## Slots

| Slot | What goes in |
|---|---|
| `{{COMPANION_CANON}}` | The companion's look, word for word, from `<memory>/appearance/companion.md` |
| `{{ART_STYLE}}` | The art style word or phrase the member chose in the companion's canon |
| `{{HAIR}}` | A full description of the hairstyle: length, cut, parting, bangs, volume, texture, color and highlights, ties, clips or ornaments |

## Prompt

```
A {{ART_STYLE}} hairstyle turnaround sheet of one character, shown three times side by side in a single image: a front view on the left, a side view (facing right) in the middle, and a back view on the right.

Character: {{COMPANION_CANON}}

Hairstyle: {{HAIR}}. This hairstyle is the subject: show its length, parting, bangs, volume, texture, color, and every tie, clip or ornament clearly in all three views, consistent from every angle. The back view shows how the hair falls and how it is tied or layered. This hairstyle wins over any hair seen in other pictures.

Identity: the same face as the portrait reference, identical in the front and side views.

Framing: head and shoulders in every view, from just above the top of the hair to the upper chest, nothing cropped, including the full length of the hair where it falls. All three heads the same size and evenly spaced. Simple, plain top or neckline so nothing distracts from the hair.

Background: plain, dark, even background (deep charcoal or near black), with no scenery and no props, so the hair's outline reads clearly.

Lighting: soft, even studio light with a gentle rim light that separates the hair from the dark background, true colors.

No text, no labels, no arrows, no watermark, no signature, no extra characters.
```

## Settings

- **Size:** `1536x1024` (landscape, room for three views)
- **Quality:** `high`

## Notes

- When `<memory>/appearance/portrait.png` exists, pass `-Reference <memory>/appearance/portrait.png` so the face and identity stay the same.
- When `CASTLE_IMAGE` is `codex`, the AI draws this with its own image tool, using the reference picture the same way, and saves to the same path.
- The hairstyle reference wins over any hair seen in other pictures: the portrait reference is for the face only - draw the hair from `{{HAIR}}`, not from the portrait.
