# Roadmap

Items are roughly priority-ordered within each section. Most work lives in
neverlight-mail-core and is shared across all frontends (COSMIC, TUI, macOS/iOS).

---

## Offline & Resilience

### Offline reading
Cache is already the UI source of truth. Make it explicit: when disconnected,
render from cache with a clear "offline" indicator. No panics, no blank screens.

### Queued mutations
Flag changes, moves, and deletes made offline get written to a local pending-ops
queue. Replay on reconnect in order, reconcile conflicts with server state.

### Queued compose / send
Compose and "send" while offline. Message is saved locally, submitted via
`EmailSubmission/set` when connectivity returns. Show queued-send status.

### Drafts
Auto-save compose state to a local drafts table (and optionally to the server's
Drafts mailbox via `Email/set` with `$draft` keyword). Resume across sessions.

---

## Sync & Performance

### Background delta sync
Wire up `Email/changes` + `Mailbox/changes` end-to-end so refreshes are
incremental, not full refetch. The core has `sync.rs` — finish the plumbing.

### Fast startup
Show cached folders + cached message list immediately on launch. Connect and
delta-sync in the background. The user should see mail within 1–2 seconds, not
after a full JMAP round-trip.

### Refresh coalescing
Already partially implemented (epoch lanes, abort handles). Finish: ensure
overlapping refresh requests never produce duplicate work or stale applies.

---

## Search

### Full-text search improvements
FTS5 index exists and works. Improvements needed:
- **Global search** — search across all folders, not just the current one.
- **Cross-account search** — unified results from all accounts.
- **Search result context** — show matched snippet, not just subject/sender.

### Server-side search
Use JMAP `Email/query` with filters for searches that exceed the local cache
(e.g., messages not yet synced). Blend with local FTS results.

### Saved searches / smart folders
Let users save filter criteria as virtual folders. JMAP filter objects map
naturally to this.

---

## Compose

### Reply All
Distinct from Reply. Populate To with original sender, Cc with all other
recipients minus self.

### Cc / Bcc fields
Not currently in the compose dialog. Add them.

### Signatures
Per-account, per-identity signature blocks. Plain text initially, markdown
stretch goal.

### Address autocomplete
Autocomplete To/Cc/Bcc from recent recipients (local history) and optionally
JMAP Contacts if the server supports it.

---

## Thread & Conversation Views

### Conversation view
Beyond collapse/expand in the message list — show a full conversation thread
in the body pane with quoted-text folding, per-message actions, and
chronological flow.

### Thread-aware actions
"Archive entire thread", "Mute thread" (suppress notifications for this
thread_id), "Mark thread read".

---

## Notifications

### Desktop notifications
Use `notify-rust` (or platform equivalent) to show new-mail notifications.
Triggered from SSE push events. Currently push drives delta sync but doesn't
surface to the user.

### Notification preferences
Per-account, per-folder rules. Don't notify on high-volume mailing list
folders. Muted threads suppress notifications.

---

## Layout & Responsiveness

### Responsive layout
Right now it only looks right full screen. The delete/archive buttons aren't
visible at smaller widths. Needs:
- Collapsible sidebar (toggle or auto-hide below a width threshold).
- Compact mode: two-pane (list + preview) or single-pane (list only, tap to open).
- Action buttons that reflow or collapse into a menu at narrow widths.

### Reading pane position
Toggle between right-side preview (current), bottom preview, and no preview
(full-screen message on select).

---

## Settings

### Settings pane
Doesn't exist yet. Needs to cover at minimum:
- Account management (add/edit/remove/reorder).
- Default identity and signature per account.
- Notification preferences.
- Cache size limits and manual cache clear.
- Theme (follow system / light / dark).
- Keyboard shortcut reference (and eventually customization).
- Sync interval / push toggle.

---

## Observability

### Structured logging
Currently weak. Needs proper INFO / DEBUG / TRACE coverage:
- Connection lifecycle and reconnect attempts.
- Sync operations (what changed, how long it took).
- Cache operations (hits, misses, size).
- JMAP request/response timing.

### Diagnostics panel polish
Exists in the sidebar. Add: sync duration, cache stats, JMAP quota usage
(`Quota/get`), request count since session start.

---

## Security & Privacy

### Remote content control
The HTML→markdown pipeline blocks remote content by default. Add a per-message
"load remote images" override for messages the user trusts.

### Phishing indicators
Flag sender vs. reply-to mismatches, display-name-only senders, and
newly-seen sender domains.

---

## Multi-Account Polish

### Unified inbox
Virtual folder showing new/unread from all accounts in one view, tagged by
account.

### Per-account health
Quota usage (`Quota/get`), connection uptime, sync lag. Surface in settings
or diagnostics.

---

## Bulk Operations

### Multi-select
Shift-click or Shift+j/k to select a range. Then bulk trash, archive,
mark-read, move, star.

### Select all / select none
For the current folder view.

---

## UX Polish

### Undo
"Message trashed" / "Message archived" toast with a timed undo action.
The optimistic-update + rollback machinery already exists — surface it.

### Empty states
What does an empty inbox look like? Empty search results? A brand-new account
with no mail? Each needs intentional design, not just a blank pane.

### Loading & error states
Skeleton screens during sync, clear retry affordances on error, phase-aware
status indicators (the `Phase` enum exists, make it visible).

### Confirmation dialogs
Permanent delete, send without subject, discard unsaved draft.

### Keyboard shortcut discoverability
`?` opens a shortcut overlay / cheat sheet.

---

## Platform-Specific (Non-Core)

These live in each frontend, not in the shared engine.

### COSMIC
- System theme following (light/dark/accent).
- COSMIC Settings integration.
- Headerbar / app menu conventions.

### TUI
- Mouse support.
- Clipboard integration.
- Terminal image protocols (Kitty / Sixel) for inline image preview.
- Alternate screen handling.

### macOS / iOS
- System notification center.
- Share sheet integration.
- Handoff / Continuity between devices.
- System Contacts integration.
- Native keychain (already via `keyring` crate).


