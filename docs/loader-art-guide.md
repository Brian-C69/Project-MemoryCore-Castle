# Guide for Codex: make Castle's start-up picture

You are making **one picture**: the background of MemoryCore Castle's start-up screen, shown full
screen for a couple of seconds each time Castle opens. Read this whole guide, make the picture, then
save it exactly as described under "Delivering it".

## What the picture is for

Castle is a private desktop home for an AI companion: a night-sky, constellation-and-castle world with
glowing blue light. The start-up screen shows this picture behind a small panel of words (the brand,
a funny line, a progress bar and what is loading). The picture sets the mood of the whole app, so it
should feel like arriving somewhere warm, quiet and a little magical.

## The specification

| | |
|---|---|
| Shape | **16:9** landscape |
| Size | **1920 x 1080** (2560 x 1440 is fine if it stays sharp; never smaller than 1920 x 1080) |
| Final file | `public/loader/loader.webp` (a real WebP, not a renamed PNG) |
| File size | aim for **under 400 KB** (WebP quality about 80-85) |
| Text | **none at all** - no words, no letters, no logo, no watermark, no signature. Castle draws its own text over the picture |
| People | none required. If your idea needs a figure, keep it small, faceless and in the upper half |

## Where the words will sit (very important)

Castle covers the picture with a dark fade from the bottom and puts its text on top:

- **The bottom third is covered by words and a dark fade.** Keep it calm and dark: soft ground, mist,
  water or the deep sky. No important detail down there.
- The words sit in a **centred column** about 640 px wide (a third of the picture's width). Keep the
  bottom-centre free of bright, busy detail so the text stays readable.
- **The subject lives in the top two thirds, near the centre** (the focal point at about 50% across,
  35-40% down).

## It will be cropped - plan for it

The picture is scaled to fill whatever screen Castle is on: a 16:9 monitor shows all of it, a 16:10
laptop or an ultrawide monitor crops the sides or the top. So:

- Keep everything that matters inside the **middle 70% of the width** and the **upper 70% of the height**.
- Let the left and right edges be easy to lose: sky, clouds, stars, soft light. No subject touching an edge.

## The look

- **Mood:** calm, hopeful, intimate. Arriving home at night. A little wonder, never scary.
- **Palette:** deep midnight blue and navy (`#050b1f` to `#12214f`), with one glowing accent of soft
  sky blue (`#8fb8ff`). Small warm touches (a lit window, a lantern, a golden star) are welcome.
  Castle's default colours are blue, so the picture should sit comfortably next to them.
- **Subject idea (use or improve):** a quiet castle or tower on a hill under a huge starry sky with
  glowing constellations drawn between the stars, a soft aurora or river of light, a lit window.
  Seen from a distance, centred slightly above middle, so there is lots of sky around it.
- **Style:** painterly, soft-focus and luminous, like a storybook night illustration. Rich depth
  (foreground, middle, far sky). Not photoreal, not flat vector, not harsh.
- **Avoid:** text of any kind, bright white areas, hard black bars, heavy noise, clutter in the
  bottom third, faces close to the camera, anything copyrighted (no existing characters or logos).

## A ready-to-use image prompt

> A wide cinematic storybook illustration, 16:9, of a small glowing castle on a distant hill under an
> enormous midnight-blue starry sky, glowing constellations linking the stars, a soft river of light
> and gentle aurora, one warm lit window, painterly and luminous, deep navy and sky-blue palette with
> tiny warm golden accents, calm and hopeful mood. The castle sits slightly above the centre with
> wide open sky around it. The lower third is soft dark mist and water, calm and empty. No text, no
> letters, no logo, no watermark, no people close to the camera.

## Delivering it

1. Generate the picture at 1920 x 1080 or larger, 16:9.
2. Save it as `public/loader/loader.webp` in the castle project folder (the folder that holds
   `package.json`). If your image tool gives a PNG or JPG, convert it to WebP - for example, with
   Python and Pillow:

   ```python
   from PIL import Image
   img = Image.open("loader.png").convert("RGB")
   img = img.resize((1920, 1080), Image.LANCZOS)   # only if it is a different size
   img.save("public/loader/loader.webp", "WEBP", quality=82, method=6)
   ```

   or with ffmpeg: `ffmpeg -i loader.png -vf scale=1920:1080 -q:v 82 public/loader/loader.webp`.
3. Check the file is under about 400 KB and opens as an image. Do not change any other file, and do
   not commit or push - just leave the file in place and tell the member where it is.

The member then rebuilds Castle (`npm run tauri build`), and the start-up screen uses the picture. To
see it without waiting for a full build, `npm run dev` shows it too.

## Optional: more funny lines

If you like, you can also write `public/loader/quips.json`: an array of up to 60 short, gentle,
funny lines (each under 120 characters) for the start-up screen to rotate through, in the spirit of
"Fluffing the cushions in CHAT." and "Persuading the loading bar to be honest." Keep them kind, silly
and safe for anyone to read. See the existing ones in `src/loader/quips.js` so you do not repeat them.
