# Local portable-pty patch

Source: crates.io portable-pty 0.9.0, MIT, original source/license retained.

Only `src/win/psuedocon.rs` is changed: load the required ConPTY DLL by absolute executable-directory path instead of searching the current workspace/PATH. Loading failures propagate as PTY errors, with no silent system-host fallback or loader panic. Original flags are unchanged. No Unix/macOS behavior changes.

Skill Studio explicitly stages the pinned Microsoft ConPTY DLL and matching OpenConsole.exe using `tools/prepare_conpty.py`. Windows 10's old system host consumes DECSET mouse modes and SGR wheel reports; the modern runtime forwards them to VT-input applications. `terminal.rs` checks both sidecars before creating a Windows PTY instead of silently reverting to a known-broken wheel path. The Windows regression uses a real child process/raw console input, not only a parser test.
