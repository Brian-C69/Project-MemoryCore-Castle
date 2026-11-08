# Standard Appearance

**Purpose:** the companion's standard portrait - front-facing, head and shoulders down to the waist, on a plain soft background. This is the identity picture every other appearance drawing follows. Save it as `<memory>/appearance/portrait.png`.

## Slots

| Slot | What goes in |
|---|---|
| `{{COMPANION_CANON}}` | The companion's look, word for word, from `<memory>/appearance/companion.md` (face, eyes, skin, build, age look, signature details) |
| `{{ART_STYLE}}` | The art style word or phrase the member chose in the companion's canon (for example "anime", "soft watercolor illustration", "semi-realistic digital painting") |
| `{{OUTFIT}}` | The outfit to wear in the portrait (the canon's default outfit if none is chosen) |
| `{{HAIR}}` | The hairstyle (the canon's default hairstyle if none is chosen) |
| `{{COMPANION_NAME}}` | The companion's name, for the file's notes only - it is not drawn |

## Prompt

```
A {{ART_STYLE}} character portrait of one person.

Character: {{COMPANION_CANON}}

Hair: {{HAIR}}. The hairstyle reference wins over any hair seen in other pictures.
Outfit: {{OUTFIT}}.

Framing: front-facing, looking at the viewer, framed from the top of the head down to the waist, centered, the whole head and both shoulders inside the frame. Relaxed, natural pose, arms at the sides or loosely in front. A gentle, friendly expression.

Background: plain, soft, evenly lit background in a light neutral tone, with no objects, no scenery, no text and no patterns.

Lighting: soft, even front light with a gentle rim light, clean shadows, true colors.

Keep the identity exact: the same face shape, eyes, eye color, skin tone, proportions and signature details as described. One person only. No text, no watermark, no signature, no frame, no extra limbs.
```

## Settings

- **Size:** `1024x1536` (portrait)
- **Quality:** `high`

## Notes

- When `<memory>/appearance/portrait.png` already exists (redrawing or refreshing the portrait), pass `-Reference <memory>/appearance/portrait.png` so the identity stays the same.
- When `CASTLE_IMAGE` is `codex`, the AI draws this with its own image tool, using the reference picture the same way, and saves to the same path.
- The hairstyle reference wins over any hair seen in other pictures: if a hairstyle turnaround exists for `{{HAIR}}`, follow it rather than the hair in the old portrait.
