# Echo

Fully local **voice dictation**: hold a keyboard shortcut, speak, and text is
typed into the active application's focused input. A small animated bubble
appears at the bottom of the screen while you speak.

<img width="592" height="877" alt="Echo" src="https://github.com/user-attachments/assets/2e09227f-adfa-428d-9ed1-5925663be1ee" />


- **Tauri 2 + Svelte 5 + Deno** on macOS, Windows, and Linux.
- **Swappable local models** depending on the use case: **Parakeet TDT 0.6B v3**
  (recommended, ONNX, ~25x real-time on CPU, 25 languages), **Whisper Small**
  (lighter, fast enough for live mode with CPU/Metal), and **Whisper Medium**
  (higher accuracy, ~100 languages). Models can be downloaded and switched from
  the settings window.
- **Multilingual** with automatic language detection or a forced language.

## How it works

1. Hold the shortcut key combination: by default **Ctrl+Alt+Space** on
   macOS/Linux and **Ctrl+Shift+Space** on Windows.
2. The bubble appears at the bottom center of the screen, animates with your
   voice, and shows the partial transcription live.
3. Text is typed **live** into the focused field in live mode, then reconciled
   on key release with the final transcription. In end mode, everything is
   typed at once when you release the shortcut.

## Development

```bash
deno install          # frontend dependencies
deno task tauri dev   # run the app (builds the Rust backend)
```

Production build: `deno task tauri build`.

## Permissions

- **macOS**: allow **Microphone** access when first prompted, and grant
  **Accessibility** access in System Settings -> Privacy & Security ->
  Accessibility. This is required to type text into other applications.
- **Linux**: simulated typing requires `libxdo` on X11. On Wayland, support
  depends on the compositor.
- **Windows**: no special permission is required.

## Notes

- Models are stored in the app data directory
  (`~/Library/Application Support/com.leomartin.echo/models` on macOS).
- If your shortcut includes a modifier key (Cmd/Ctrl/Alt/Shift), prefer the
  end insertion mode: typing while the modifier is still physically held down
  can trigger shortcuts in the target application.
- Whisper does not stream natively. Partial results are decoded from a sliding
  window of up to about 8 seconds roughly every second. When you pause speaking,
  the decoded text is committed and the window restarts, which keeps latency
  stable even during long dictation sessions. Only the stable part between two
  decodes is typed live, and the final decode fixes the tail end.
