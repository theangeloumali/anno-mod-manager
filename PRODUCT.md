# PRODUCT

> Inferred from Angelo's brief (2026-11-19) and `claudedocs/research/`. Lines marked _(assumed)_ are not yet confirmed.

## Product

A free, public desktop mod manager for **Anno 117: Pax Romana** and **Anno 1800**. It browses and installs mods from mod.io, installs dropped archives and folders automatically, manages activation, profiles and load order, and explains what will break before the game is launched.

Working name: **Atelier**. _(assumed, easy to rename)_

## Audience

Anno players on Windows (Steam, Ubisoft Connect, Epic), and Linux/Steam Deck through Proton. They range from first-time modders to collection curators running 100+ mods. They use it in the minutes before a play session, often in a dim room with the game launcher beside it.

## Jobs

1. Get a mod from mod.io or a downloaded zip running in-game without touching the file system.
2. Know before launch whether the active set is broken (missing dependency, incompatibility, wrong game, unsafe for this save).
3. Switch between mod sets (vanilla, campaign, multiplayer parity) in one click.
4. Keep mods updated without losing what was active.

## Platform

Desktop (Tauri 2 + React/TypeScript). Windows first, Linux second. Minimum window 960×600.

## Constraints

- mod.io API terms and "Powered by mod.io" attribution apply. The app needs its own API key for the public release.
- No Ubisoft or Anno art, logos or trademarks in the UI. The world is evoked, not copied.
- The game owns load order; the app explains it but cannot override it.
- Writes to the mods folder are blocked while the game is running.

## Success

A dropped archive goes from drop to active in under 10 seconds, with a preview the user trusts. Zero "mods shown active but not loaded" reports. _(assumed metrics)_
