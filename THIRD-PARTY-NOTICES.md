# Bundled fonts

The native application embeds Geist (UI) and JetBrains Mono regular/italic (editor). These font files were obtained from the local Herdr checkout's `assets/fonts/` directory and are distributed under the SIL Open Font License, Version 1.1.

The original copyright and license texts are preserved at:

- `gpui/assets/fonts/OFL.txt` — Geist
- `gpui/assets/fonts/OFL-JetBrainsMono.txt` — JetBrains Mono

The graphite/ink/moss/paper design palettes and semantic color calculations were aligned with Herdr's `src/settings.rs`. The terminal's independent GPUI surface, input-method composition model and bounded-output approach were informed by Herdr (`herdr-app`, Apache-2.0). This implementation uses its own local PTY/VT integration; it does not bundle herdr-core, libghostty-vt, Herdr's JavaScript mods, or its execution runtime.

# Native terminal dependencies

- `portable-pty` 0.9.0 — MIT; WezTerm project.
- `vt100` 0.16.2 — MIT; https://github.com/doy/vt100-rust.

Original license texts and the Herdr reference license are preserved under `gpui/assets/licenses/` and included in packaged distributions.
