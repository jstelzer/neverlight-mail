# Claude Context: Nevermail (cosmic-email)

**Last Updated:** 2025-02-23

## What This Is

Nevermail is a COSMIC desktop email client built on:
- **libcosmic** (git, HEAD) — COSMIC UI framework (iced fork)
- **melib 0.8.13** — Mail engine from the meli project (IMAP, MIME parsing, envelope handling)

Target server: Runbox (mail.runbox.com:993, implicit TLS). Should work with any standard IMAP server.

## Critical: Version Pinning

melib 0.8.13's `imap` feature depends on `imap-codec` and `imap-types`. Newer alpha versions of these crates introduced a breaking change (missing `modifiers` field) that prevents compilation.

**The lockfile pins these to working versions:**
- `imap-codec = 2.0.0-alpha.4`
- `imap-types = 2.0.0-alpha.4`

**DO NOT run `cargo update` without verifying these pins are preserved.** If they drift, re-pin with:
```bash
cargo update -p imap-codec --precise 2.0.0-alpha.4
cargo update -p imap-types --precise 2.0.0-alpha.4
```

This is an upstream melib bug. Monitor melib releases for a fix.

## Architecture

```
src/
├── main.rs          — Entry point, env_logger init, cosmic::app::run
├── app.rs           — AppModel + Message enum, async task wiring
├── config.rs        — Config struct, Config::from_env()
├── core/
│   ├── imap.rs      — ImapSession: connect, fetch_folders, fetch_messages, fetch_body
│   ├── mime.rs      — render_body (text/plain preference, html2text fallback), open_link
│   ├── models.rs    — Folder, MessageSummary, MessageBody, Attachment, ConnectionState
│   └── store.rs     — SQLite stubs (not wired yet)
└── ui/
    ├── sidebar.rs   — Folder list view
    ├── message_list.rs — Message header list view
    └── message_view.rs — Message body preview pane
```

## Key Design Decisions

### COSMIC Task Pattern
COSMIC's `Task<M>` is `iced::Task<cosmic::Action<M>>`. You cannot use `Task::perform()` directly with app messages. Use `cosmic::task::future()` instead, which auto-wraps via the blanket `impl<M> From<M> for Action<M>`:
```rust
cosmic::task::future(async move {
    Message::FoldersLoaded(session.fetch_folders().await)
})
```

### ImapSession Design
- Wraps `Arc<Mutex<Box<ImapType>>>` for interior mutability (`fetch()` requires `&mut self`)
- `ImapSession` itself lives behind `Arc<ImapSession>` so it can be cloned into async tasks
- melib's `ResultFuture<T>` is `Result<BoxFuture<'static, Result<T>>>` — double-unwrap pattern:
  ```rust
  let future = backend.mailboxes().map_err(/*...*/)?;  // outer Result
  let result = future.await.map_err(/*...*/)?;          // inner Result
  ```
- Streams from `fetch()` are `'static` — safe to drop the lock before consuming

### Async Flow
```
init() → Config::from_env() → ImapSession::connect()
  → Connected(Ok) → fetch_folders()
    → FoldersLoaded(Ok) → auto-select INBOX → fetch_messages()
      → MessagesLoaded(Ok) → display in list

SelectFolder(i) → fetch_messages(mailbox_hash)
SelectMessage(i) → fetch_body(envelope_hash) → render via mime::render_body()
```

### MIME Body Extraction
Walks the attachment tree recursively looking for text/plain and text/html parts. Uses `Attachment::decode(Default::default())` for content-transfer-encoding. Prefers text/plain; falls back to html2text on text/html.

## Credentials (Phase 0)

Environment variables, no UI prompt:
```bash
export NEVERMAIL_SERVER=mail.runbox.com
export NEVERMAIL_PORT=993        # optional, default 993
export NEVERMAIL_USER=you@runbox.com
export NEVERMAIL_PASSWORD=yourpassword
export NEVERMAIL_STARTTLS=false  # optional, default false (implicit TLS)
```

Config::from_env() panics with a helpful message if required vars are missing.

## melib API Quick Reference

Key types and their locations in melib 0.8.13:
- `AccountSettings` — `melib::conf`, extra config via `IndexMap<String, String>` (flattened serde)
- `ImapType::new(&AccountSettings, IsSubscribedFn, BackendEventConsumer) -> Result<Box<Self>>`
- `MailBackend::mailboxes() -> ResultFuture<HashMap<MailboxHash, Mailbox>>`
- `MailBackend::fetch(&mut self, MailboxHash) -> ResultStream<Vec<Envelope>>`
- `MailBackend::envelope_bytes_by_hash(EnvelopeHash) -> ResultFuture<Vec<u8>>`
- `Mail::new(bytes, flags) -> Result<Mail>` then `mail.body() -> Attachment`
- `Envelope`: `.subject()`, `.from()`, `.date_as_str()`, `.is_seen()`, `.flags().is_flagged()`, `.has_attachments`, `.hash() -> EnvelopeHash`
- `BackendMailbox` (trait behind `Mailbox` type alias): `.name()`, `.path()`, `.count() -> (total, unseen)`, `.hash()`
- `IsSubscribedFn`: `Arc<dyn Fn(&str) -> bool + Send + Sync>.into()`
- `BackendEventConsumer::new(Arc<dyn Fn(AccountHash, BackendEvent) + Send + Sync>)`
- Hash types: `MailboxHash(pub u64)`, `EnvelopeHash(pub u64)` — transparent newtypes

## Known Limitations (Phase 0)

- **No local cache** — full envelope download on every startup (slow for large mailboxes)
- **No pagination** — fetches all messages in a folder
- **No credential UI** — env vars only
- **No SMTP** — read-only (lettre dep exists but isn't wired)
- **No search, no threading, no attachment download**

## Phase Roadmap

- **Phase 0** (done): Connect to IMAP, list folders, display message headers, render body
- **Phase 1**: SQLite cache (store.rs), incremental sync, pagination
- **Phase 2**: Credential management (keyring or config file)
- **Phase 3**: SMTP compose/send via lettre
- **Phase 4**: Search, threading, attachment handling, multi-account (per-account config/cache, account switcher UI)
