# Story Cast

**Purpose:** one portrait per story character (the hero, an NPC) for the STORY stage, which shows it whichever look the member chose - standing half-body over the scene (Classic VN), as a big face in a framed window (JRPG), or cut round (Cinematic, Storybook, Messenger). One picture serves them all. Save it as `<world>/cast/<name>.png` (the name as written in `setting.md`, lowercase with hyphens; `hero.png` for the hero), 1024x1536, with `-Transparent` so the scene shows around them.

The companion needs no picture here: the stage uses its standard appearance (`appearance/portrait.png`).

## Slots

| Slot | What goes in |
|---|---|
| `{{CHARACTER}}` | The character's look from `setting.md`: age look, build, face, hair, eyes, clothes and gear for their class or role, one signature detail |
| `{{MOOD}}` | Their usual expression (calm, fierce, kind, sly, weary ...) |
| `{{ART_STYLE}}` | The art style the world's scene pictures use (the same words every time, so the cast matches the scenes) |

## Prompt

```
A {{ART_STYLE}} character portrait for a visual novel: one person, {{CHARACTER}}.
Half-body, from the top of the head to just below the waist, facing slightly to the side, looking toward the viewer, the whole head and both shoulders inside the frame, centred with a little space above the head.
Expression: {{MOOD}}. Relaxed, natural standing pose, hands visible or resting.
Background: fully transparent - no scenery, no floor, no shadow, no frame, no text.
Lighting: soft, even, with a gentle rim light so the figure stands out on any scene.
```

## Rules

- One character per picture, never two.
- Keep the face clear and large enough to crop round for the small faces.
- The same character keeps the same picture for the whole adventure; paint again only if their look truly changes (a new outfit after a chapter, a transformation).
