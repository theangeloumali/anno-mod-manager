# Anno Mod Manager

A desktop mod manager for **Anno 117: Pax Romana**, **Anno 1800** and **Humankind**.

- Browse, subscribe to and update mods from mod.io.
- Drop a zip, 7z or mod folder on the window. It finds every mod inside, checks it, and shows you a preview before anything is written.
- Turn mods on and off per profile, see the predicted load order and why, and get warned about missing dependencies and conflicts before you launch.
- Humankind: set the load order yourself within the game's rules. Launch is blocked while the list names a mod that isn't installed, because the game would reset the whole list. Restore the exact mods and versions a save was played with, and install BepInEx in one click.

Built with Tauri 2 (Rust) and React with shadcn/ui in the ZKidz theme.

> Status: early development. Not affiliated with Ubisoft, Amplitude Studios, SEGA or mod.io.

## Development

```bash
pnpm install
pnpm dev            # UI in the browser with mock data
cargo test -p atelier-core -p atelier-modio -p atelier-install -p atelier-app
pnpm test           # UI logic and mock rules (node --test)
pnpm tauri dev      # full desktop app (needs the Tauri system dependencies)
```

Put a test.mod.io API key in `.env.local` as `MODIO_API_KEY=...` to run the live mod.io smoke test.

## Licence

All rights reserved. See [LICENSE](LICENSE).
