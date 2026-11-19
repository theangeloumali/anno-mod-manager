# DESIGN

**Theme: ZKidz** (Angelo, 2026-11-19). The app uses the same tokens as ZKidz Studio and ZKidz Courses on shadcn/ui. The v1 "harbour ledger" direction in `mockup/index.html` is **superseded**. That file is kept only as the reference for flows and edge states.

## Source of truth

- `src/styles/tokens.css` is copied verbatim from `zkidz-studio/design/tokens.css`, which is byte-identical to `zkidz-courses/styles/zkidz-tokens.css`. Change it upstream, never here.
- `src/styles/app.css` is Studio's Tailwind 4 `@theme inline` layer. It maps shadcn's semantic names (`background`, `primary`, `card` and so on) onto the ZKidz tokens.
- `components.json` uses shadcn `base-nova` (Base UI primitives) with lucide icons, the same as Studio.

## Rules

- **Grounds:** navy-slate dark (`#0c1117`) is the default; cream light (`#f3efe8`) comes through `data-theme="light"`.
- **Gold (`--color-cta`)** is for the one primary action per view (Install, Launch, Subscribe) and nothing else. Steel (`--color-accent`) marks links, active state, progress and focus.
- **Type:** Archivo variable (with the width axis, so `font-condensed` works for dense tables), plus JetBrains Mono for ModIDs, versions, paths and sizes. Both are self-hosted through fontsource, so the app works offline.
- **Shapes:** pill controls (`--radius-control`), 1.25rem panels (`--radius-object`), 0.375rem fields.
- **Game identity:** the theme stays ZKidz for both games. The game switch changes only a small game badge and per-game copy, never the palette.
- **States** are shadcn `Badge` variants plus an icon, never colour alone (WCAG 2.2 AA).
- **Motion:** `--ease-out`, `--dur-1` for presses and toggles and `--dur-2` for sheets. Respect reduced motion.
