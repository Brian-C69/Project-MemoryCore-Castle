# Outfit Turnaround

**Purpose:** a full-body character sheet of the companion wearing one outfit - front, side and back views in one image on plain white - so the outfit can be drawn the same way every time.

## Slots

| Slot | What goes in |
|---|---|
| `{{COMPANION_CANON}}` | The companion's look, word for word, from `<memory>/appearance/companion.md` |
| `{{ART_STYLE}}` | The art style word or phrase the member chose in the companion's canon |
| `{{OUTFIT}}` | A full description of the outfit: every garment, colors, materials, footwear, accessories |
| `{{HAIR}}` | The hairstyle to wear (the current hairstyle, or the canon's default) |

## Prompt

```
A {{ART_STYLE}} character turnaround sheet of one character, shown three times side by side in a single image: a front view on the left, a side view (facing right) in the middle, and a back view on the right.

Character: {{COMPANION_CANON}}

Outfit: {{OUTFIT}}. Show every part of the outfit clearly in all three views, with the same colors, materials, layers, footwear and accessories in each view.

Hair: {{HAIR}}. The hairstyle reference wins over any hair seen in other pictures.

Identity: the same face and hair as the portrait reference, identical in all three views.

Framing: full body from the top of the head to the soles of the feet in every view, nothing cropped. Neutral standing pose, arms slightly away from the body so the outfit's sides are visible, feet together. All three figures the same height and scale, evenly spaced, standing on the same ground line.

Background: plain pure white, with no floor shadow pattern, no scenery, no props.

Lighting: flat, even studio light, true colors, clean lines.

No text, no labels, no arrows, no watermark, no signature, no extra characters.
```

## Settings

- **Size:** `1536x1024` (landscape, room for three views)
- **Quality:** `high`

## Notes

- When `<memory>/appearance/portrait.png` exists, pass `-Reference <memory>/appearance/portrait.png` so the face and identity stay the same.
- When `CASTLE_IMAGE` is `codex`, the AI draws this with its own image tool, using the reference picture the same way, and saves to the same path.
- The hairstyle reference wins over any hair seen in other pictures: if the portrait shows a different hairstyle than `{{HAIR}}`, draw `{{HAIR}}`.
