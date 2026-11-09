# Worlds

The 3D worlds that ship with Castle. Each folder holds a world's prepared pieces (GLB, meshopt-compressed) and its card picture; the world itself is described in `src/worlds/<id>/world.json`.

| World | Pieces |
|-------|--------|
| `crystal-isle/` | The base isle, the great crystal and three orbiting islets - 3D models made with Tripo for MemoryCore Castle |
| `memory-isles/` | Waiting for its pieces |
| `floating-isles/` | The card picture only - this world is drawn from geometry |

Anything you add here ships with your build, so use models you have the rights to share. To prepare your own, see MAKE-IT-YOURS.md ("A world of your own"). While tuning a world, open the dev page with `?world=3d/<id>&camera=orbit` to see it in the browser.
