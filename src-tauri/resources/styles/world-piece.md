# World Piece

**Purpose:** one picture per piece of a world, made so a 3D tool (Tripo or similar) can turn it into a model. One isolated object, whole, on a plain background. Save each as `<memory>/worlds/<slug>/design/<piece file with .png instead of .glb>`, 1024x1024 (for example `01-base-isle.png`).

## Slots

| Slot | What goes in |
|---|---|
| `{{PIECE}}` | What this piece is, from `world.json` and the member's words. Base isle: "a large floating island: rocky underside tapering to a point with hanging roots, a wide flat grassy top, the centre left EMPTY and flat (the centrepiece stands there as its own piece)". Centrepiece: the thing at the heart of the world, standing upright on its own, "no ground, no island, no pedestal". Islet: "a small floating islet: rock underside tapering to a point, grassy top with ...". Isle (archipelago): one island and the building or garden it holds |
| `{{STYLE}}` | The same look as the concept picture (pass `design/00-concept.png` with `-Reference` when it exists) |

## Prompt

```
A single isolated stylized fantasy game asset for 3D modelling, shown whole in a three-quarter isometric view from slightly above, centered with empty space around it, on a plain flat light-gray background, no ground shadow, no other objects, no sky, no clouds behind it.
{{PIECE}}
Clean readable silhouette, simple solid shapes, {{STYLE}}, soft even studio light. No text, no letters, no watermark.
```

## Rules

- One piece per picture, never two. A piece that should stand on another (the centrepiece on the base) is drawn alone.
- Keep the base's centre empty: Castle stands the centrepiece there itself.
