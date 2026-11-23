# Terminal display reconstruction

## Delivered

- Separate terminal typography: bundled JetBrains Mono, default 14 px, 10–26 px controls and reset in the terminal's ⋯ menu. Editor font size remains independent. Existing settings files migrate through defaults.
- Read ascent, descent, gap and advance from the actual bundled font with `ttf-parser`. Snap cell size and baseline to device pixels; use the same metrics for PTY rows/columns, painting, selection, scrollbar and IME placement. Compact height is based on face metrics; the menu also offers 1.15× spacing.
- Disable discretionary/contextual text ligatures and snap glyph clusters to integer cell positions. Clone GPUI's cached shaping layout before adjustment. Preserve combining-mark offsets; align fallback fonts to the same baseline, fit/center non-ASCII glyphs, and retain wide-character selection boundaries.
- Connect common single-line TUI borders directly at cell edges. ANSI colors, truecolor, inverse/dim styles, alternate screen, existing scroll routing and bounded row caches remain in place. Respect block/underline/bar DECSCUSR cursor shapes; an unfocused cursor is outlined. Blink variants currently share the corresponding static shape.
- A 30 px toolbar shows shell/status, maximize/restore, menu and collapse. Startup directory is a tooltip, not a claim about live cwd. Clipboard, scroll compatibility, appearance, restart and confirmed stop live in the scrollable menu. Normal sessions do not spend a row on a permanent status footer.
- Maximize uses the whole Document region, not the entire workbench. It retains the same PTY, document entities, unsaved buffers and dock height. Explicitly opening a source file restores the document split instead of focusing an invisible editor. Hiding still leaves the process running.

## Verification

- `cargo test --locked`: 97 application tests, including device-pixel metrics at 100/125/150/200%, shaping-cache isolation, combining-mark offsets, wide-character selection, cursor-style parsing and independent preference migration/validation.
- Windows native GUI smoke with isolated temporary workspace/configuration: Chinese + combining acute + basic emoji, fixed-width ASCII/ligature samples, connected borders, truecolor, terminal-only font change, maximize/restore with session variable and cwd intact, automatic source reveal, unsaved document preservation, and clean owned-window exit. Original source on disk remains unchanged until the test explicitly saves its own document.
- Installed Codex CLI 0.153.4: isolated `CODEX_HOME`, cleared API-key environment, startup/login TUI, arrow-key navigation, dock/maximize redraw and cancellation back to the shell. No login, model prompt or authenticated agent/tool execution was performed. This is not a blanket Agent CLI compatibility claim.
- Windows 10 19044 wheel regression: the old system ConPTY swallowed both DECSET mouse requests and SGR reports. The pinned Microsoft ConPTY runtime preserves modes and delivers a real SGR wheel event to child raw stdin; a process-level regression asserts the captured bytes. Native GUI smoke launched the actual local Pi CLI in offline fullscreen mode with isolated configuration and no MCP/tools/session. Physical Win32 wheel events scrolled Pi's `/hotkeys` transcript while its editor/footer stayed fixed; no model request was made.
- Windows packaging stages `conpty.dll` and matching `OpenConsole.exe` from Microsoft.Windows.Console.ConPTY 1.25.261109001, with package SHA-256 verification. The local portable-pty patch uses an absolute executable-directory DLL path; neither a workspace DLL nor a silent old-system-host fallback is used for application sessions. Run `python tools/prepare_conpty.py --profile debug` / `--profile release` before Windows tests/builds. Existing running sessions retain their original host until a new application process is launched.
- macOS interaction and actual IME candidate/composition behavior still require native manual verification. High-volume output is bounded by the existing PTY/queue/history design, not a new throughput guarantee.

`SKILL_STUDIO_CONFIG_DIR` optionally points at an isolated settings directory for native smoke tests. Without it, the normal user configuration location is unchanged.

## Ghostty assessment — not integrated

The local Herdr reference separates a GPUI terminal surface (`src/terminal.rs`), a Ghostty VT wrapper (`src/vt.rs`), and daemon/transport integration (`src/hc.rs`). Its Ghostty build uses `crates/ghostty-sys/build.rs` and a vendored Ghostty submodule. In this environment that submodule is uninitialized and Zig is not on PATH; libclang/toolchain availability has not been validated. Its patched GPUI/font APIs are not all available in our pinned upstream GPUI version.

A future migration should preserve our owned local `portable-pty` process and safety guards, replace the VT screen adapter, and separately validate Windows/macOS builds, Unicode/graphemes, historical-line reflow, mouse protocols, queries and resize behavior. Pulling in all of `herdr-core` is not required for local rendering. No Zig toolchain, Ghostty checkout, daemon or remote execution service was installed by this reconstruction.

The current `vt100` engine remains a deliberate boundary: font changes/column resizing do not provide Ghostty-style full logical-line reflow; complex ZWJ emoji and advanced terminal protocols remain limited. Known Windows Ctrl+C behavior is unchanged, with confirmed stop/restart as fallback. Font/display improvements do not resolve those engine or console limitations.

Herdr attribution and third-party license texts are listed in `THIRD-PARTY-NOTICES.md` and `gpui/assets/licenses/`.
