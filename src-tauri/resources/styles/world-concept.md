# World Concept

**Purpose:** one wide picture of the whole world the member asked for, for their eye only (it is the world's card in Castle's WORLD room). Save it as `<memory>/worlds/<slug>/design/00-concept.png`, 1536x1024.

## Slots

| Slot | What goes in |
|---|---|
| `{{WORLD_WORDS}}` | The member's words from `request.md`, cleaned into one or two sentences |
| `{{LAYOUT}}` | For a centrepiece: "one great floating island with a centrepiece at its heart, three small islets drifting around it in a slow circle". For an archipelago: "five floating islands joined by glowing bridges of light, a few small islets between them" |
| `{{PIECES}}` | What each piece in `world.json` is, in a few words each, in order |
| `{{STYLE}}` | The look: the member's picture if they gave one (pass it with `-Reference`), else "hand-painted stylized fantasy, clean shapes" |

## Prompt

```
Wide cinematic fantasy concept art of {{LAYOUT}}, floating in a calm sky above a sea of clouds.
The world: {{WORLD_WORDS}}
What is in it: {{PIECES}}
Soft magical light, gentle glow at the heart of the world, a few loose floating rocks, calm and wondrous mood.
{{STYLE}}. No text, no letters, no watermark.
```
