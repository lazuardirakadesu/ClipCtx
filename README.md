# ClipCtx 📋✨
**Context-Aware Windows 11 Clipboard Manager**

ClipCtx is a lightweight, high-performance Windows desktop clipboard manager built with **Tauri v2 (Rust)**, **React 19**, **TypeScript**, **Tailwind CSS v4**, and **SQLite (FTS5)**. It provides a Spotlight-style command palette interface with native Windows 11 Mica/Acrylic glass vibrancy, active window context extraction, privacy guards, smart content classification, and zero-polling clipboard synchronization.

---

## 🌟 Key Features

- **Native Windows 11 Fluent Vibrancy:**
  - True Mica backdrop (`window-vibrancy`) with acrylic fallback on Windows 10.
  - Specular 1px Fluent borders, soft multi-layer drop shadows, and adaptive theme switching.
- **Zero-Polling Win32 Clipboard Listener:**
  - Dedicated background thread creates a hidden Win32 message-only window (`HWND_MESSAGE`).
  - Listens directly for `WM_CLIPBOARDUPDATE` via `AddClipboardFormatListener` without CPU-heavy loops or `setInterval`.
- **Active Window Context Extraction:**
  - Extracts the foreground executable name (`Code.exe`, `chrome.exe`, `WindowsTerminal.exe`, etc.) via `GetWindowThreadProcessId` + `QueryFullProcessImageNameW`.
  - Captures the exact active window title with `GetWindowTextW`.
- **Privacy & Deduplication Guards:**
  - Automatic filtering of password managers (`Bitwarden.exe`, `1Password.exe`, `KeePass.exe`, etc.).
  - Automatic suppression of incognito / in-private browsing clipboard items.
  - SHA-256 deduplication: repeated clips update `last_used_at` rather than duplicating rows.
- **Smart Content Classification:**
  - Heuristic classifier detects `url`, `color` (HEX, RGB, HSL), `json`, `code`, and `text`.
  - Interactive inspector renders live color swatches, formatted JSON trees, code blocks with line counts, and link previews.
- **Instant SQLite FTS5 Full-Text Search:**
  - High-speed local database located in `%LOCALAPPDATA%/clipctx/clipctx.db`.
  - FTS5 virtual table with insert/update/delete triggers for sub-millisecond keyword matching.
- **Global Shortcut & Win32 Auto-Paste:**
  - Global `Alt+V` hotkey opens or toggles the Spotlight UI.
  - Automatically captures the previous foreground `HWND` and simulates `Ctrl+V` using Win32 `SendInput` upon pressing `Enter`.
- **System Tray Integration:**
  - Windows notification area icon with quick actions: `Open ClipCtx (Alt+V)`, `Clear Unpinned History`, and `Quit`.

---

## ⌨️ Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Alt + V` | Summon / Toggle ClipCtx popup window from any application |
| `↑` / `↓` | Navigate through clipboard history items |
| `Enter` | Paste selected clip directly into the previously active window |
| `Space` | Copy selected clip to clipboard without pasting |
| `Ctrl + P` | Pin / unpin selected clip |
| `Delete` | Remove selected clip from history |
| `Esc` | Dismiss / hide ClipCtx window |

---

## 🏗️ Architecture

```
CLIPCTX/
├── src-tauri/                     # Rust backend (Tauri v2)
│   ├── src/
│   │   ├── classifier.rs         # Smart content type classifier (URL, Color, JSON, Code, Text)
│   │   ├── clipboard_listener.rs # Hidden HWND_MESSAGE thread & Win32 active context extractor
│   │   ├── db.rs                 # SQLite connection, schema, FTS5 virtual table & CRUD operations
│   │   ├── models.rs             # Data structures (ClipItem, AppFilterItem, ContentType)
│   │   ├── paste.rs              # HWND tracking, focus restoration & SendInput Ctrl+V simulation
│   │   ├── tray.rs               # Windows system tray builder & menu handlers
│   │   ├── lib.rs                # App entrypoint, global shortcuts, Mica vibrancy, Tauri commands
│   │   └── main.rs               # Entry point
│   ├── Cargo.toml                # Dependencies (rusqlite, window-vibrancy, windows, arboard, sha2)
│   └── tauri.conf.json           # Window configuration (transparency, borderless, alwaysOnTop)
├── src/                           # React 19 Frontend
│   ├── components/
│   │   ├── AppFilterBar.tsx      # Dynamic horizontal app context filter pills
│   │   ├── ClipListItem.tsx      # Fluent list item with accent pill & relative timestamp
│   │   ├── InspectorPreview.tsx  # Detailed split-pane preview (color swatch, code, link cards)
│   │   └── SearchBar.tsx         # Borderless search bar with shortcut badges
│   ├── utils/
│   │   └── formatters.ts         # Relative time, HEX/RGB/HSL conversion & JSON formatting
│   ├── types.ts                  # Shared TypeScript models
│   ├── App.tsx                   # Main Spotlight Palette layout & keyboard controller
│   └── App.css                   # Tailwind CSS v4 directives & Windows 11 fonts
└── vite.config.ts                # Vite with @tailwindcss/vite plugin
```

---

## 🚀 Building & Running

### Prerequisites
- Windows 10 or Windows 11 (64-bit)
- [Node.js](https://nodejs.org/) (v18+)
- [Rust toolchain](https://rustup.rs/) (stable MSVC)

### Development
```bash
# Install frontend dependencies
npm install

# Run in development mode (hot-reloads React frontend + Tauri backend)
npm run tauri dev
```

### Production Build
```bash
# Build frontend and compile optimized release binary
npm run build
npm run tauri build
```
The compiled standalone executable will be located in:
`src-tauri/target/release/clipctx.exe`
