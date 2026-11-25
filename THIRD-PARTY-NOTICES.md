# Bundled fonts

The native application embeds Geist (UI) and JetBrains Mono regular/italic (editor). These font files were obtained from the local Herdr checkout's `assets/fonts/` directory and are distributed under the SIL Open Font License, Version 1.1.

The original copyright and license texts are preserved at:

- `gpui/assets/fonts/OFL.txt` — Geist
- `gpui/assets/fonts/OFL-JetBrainsMono.txt` — JetBrains Mono

The graphite/ink/moss/paper design palettes and semantic color calculations were aligned with Herdr's `src/settings.rs`. The terminal's independent GPUI surface, input-method composition model, device-pixel font-metric/baseline approach and bounded-output approach were informed by Herdr (`herdr-app`, Apache-2.0). This implementation uses its own local PTY/VT integration; it does not bundle herdr-core, libghostty-vt, Herdr's JavaScript mods, or its execution runtime.

# Native terminal dependencies

- `portable-pty` 0.9.0 — MIT; WezTerm project. Source vendored under `gpui/vendor/portable-pty`, with the Windows DLL loader restricted to the executable directory; see its `SKILL-STUDIO-PATCH.md`.
- `vt100` 0.16.2 — MIT; https://github.com/doy/vt100-rust.
- `ttf-parser` 0.25.1 — MIT OR Apache-2.0 (MIT license included); https://github.com/harfbuzz/ttf-parser. Used to read the bundled terminal font's metrics; no runtime font downloads.

- Microsoft.Windows.Console.ConPTY 1.25.261109001 — MIT; Microsoft Corporation, https://github.com/microsoft/terminal. Windows distributions include the matching `conpty.dll` and `OpenConsole.exe` from the official NuGet package; `tools/prepare_conpty.py` verifies its pinned SHA-256. No system console installation is replaced.

Original license texts and the Herdr reference license are preserved under `gpui/assets/licenses/` and included in packaged distributions.
