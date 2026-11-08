# Try-On

**Purpose:** the standard portrait, redrawn wearing a chosen outfit with a chosen hairstyle - a quick look at "how would this suit me?" before the member keeps it.

## Slots

| Slot | What goes in |
|---|---|
| `{{COMPANION_CANON}}` | The companion's look, word for word, from `<memory>/appearance/companion.md` |
| `{{ART_STYLE}}` | The art style word or phrase the member chose in the companion's canon |
| `{{OUTFIT}}` | The outfit to try on: every garment, colors, materials, accessories visible down to the waist |
| `{{HAIR}}` | The hairstyle to wear with it |

## Prompt

```
A {{ART_STYLE}} character portrait of one person, matching the standard portrait exactly in framing, pose, background and identity - only the outfit and hairstyle change.

Character: {{COMPANION_CANON}}

Outfit: {{OUTFIT}}. Show the outfit clearly from the neckline down to the waist, with its true colors, materials, layers and accessories.

Hair: {{HAIR}}. The hairstyle reference wins over any hair seen in other pictures, including the portrait reference.

Identity: the same face, eyes, eye color, skin tone and proportions as the portrait reference.

Framing: front-facing, looking at the viewer, framed from the top of the head down to the waist, centered, the whole head and both shoulders inside the frame. Relaxed, natural pose. A gentle, friendly expression.

Background: plain, soft, evenly lit background in a light neutral tone, with no objects, no scenery, no text and no patterns.

Lighting: soft, even front light with a gentle rim light, clean shadows, true colors.

One person only. No text, no watermark, no signature, no frame, no extra limbs.
```

## Settings

- **Size:** `1024x1536` (portrait, same as the standard portrait)
- **Quality:** `medium` (a try-on is a quick look; use `high` when the member keeps it)

## Notes

- When `<memory>/appearance/portrait.png` exists, pass `-Reference <memory>/appearance/portrait.png` so the identity stays the same. If a turnaround of this outfit or hairstyle exists, it may be passed as the reference instead when the helper takes only one.
- When `CASTLE_IMAGE` is `codex`, the AI draws this with its own image tool, using the reference picture the same way, and saves to the same path.
- The hairstyle reference wins over any hair seen in other pictures: the portrait reference sets the face, `{{HAIR}}` (and its hair turnaround, if any) sets the hair.
