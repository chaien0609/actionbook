---
"@actionbookdev/cli": minor
---

Add Electron app automation and pydoll-inspired browser automation features

**Electron App Automation:**
- New `actionbook app` command for automating Electron desktop apps (VS Code, Slack, Discord, Figma, Notion, Spotify)
- Auto-discover and launch apps with `app launch <name>`
- Connect to running apps with `app attach <port>`
- Full feature parity with browser commands (all 35+ commands work with app prefix)

**Pydoll-Inspired Browser Features:**
- Shadow DOM Support: Use `::shadow-root` selector syntax to interact with web components
- IFrame Context Switching: Switch between main frame and iframes with `browser switch-frame`
- Keyboard Hotkeys: Send keyboard combinations with `browser hotkey "Control+C"`
- Scroll with Wait: Wait for scrollend event with `browser scroll down 500 --wait`

**Bug Fixes:**
- Fix iframe context switching to properly execute in target frame using CDP isolated worlds
- Fix extension close command to prevent zombie processes
- Fix Windows wildcard path expansion for app discovery

All browser automation features are available in both `browser` and `app` commands.
