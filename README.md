# Nevermail

A COSMIC desktop email client for Linux, built in Rust.

**Status:** Pre-alpha (scaffolding)

## Stack

| Component   | Crate                                            | Role                                  |
|-------------|--------------------------------------------------|---------------------------------------|
| UI          | [libcosmic](https://github.com/pop-os/libcosmic) | COSMIC desktop toolkit (iced-based)   |
| Mail engine | [melib](https://crates.io/crates/melib)          | IMAP, MIME parsing, threading         |
| SMTP        | [lettre](https://crates.io/crates/lettre)        | Outbound mail delivery                |
| HTML render | [html2text](https://crates.io/crates/html2text)  | Plain-text conversion for HTML emails |
| Sanitizer   | [ammonia](https://crates.io/crates/ammonia)      | HTML sanitization                     |
| Cache       | [rusqlite](https://crates.io/crates/rusqlite)    | Local SQLite message cache            |
| Async       | [tokio](https://crates.io/crates/tokio)          | Async runtime                         |

## Architecture

```
src/
├── main.rs          Entry point
├── app.rs           COSMIC Application (MVU model + update + view)
├── config.rs        Account configuration
├── core/
│   ├── models.rs    Domain types (Folder, MessageSummary, MessageBody)
│   ├── imap.rs      melib IMAP wrapper
│   ├── store.rs     SQLite cache layer
│   └── mime.rs      HTML-to-text rendering, link handling
└── ui/
    ├── sidebar.rs       Folder list
    ├── message_list.rs  Message header list
    └── message_view.rs  Message body preview
```

The app follows the COSMIC MVU (Model-View-Update) pattern:
- **Model**: `AppModel` holds all state (folders, messages, selection, sync status)
- **View**: Three-pane layout (sidebar | message list | message preview)
- **Update**: `Message` enum drives state transitions

Data flows: IMAP (via melib) -> domain models -> SQLite cache -> COSMIC widgets.

## Roadmap

- **Phase 0**: Connect to IMAP, list INBOX headers in the UI
- **Phase 1**: Readable client (cache, message preview, attachments)
- **Phase 2**: Quality of life (threading, flags, archive/delete, keyboard shortcuts)
- **Phase 3**: Compose + send (SMTP via lettre)
- **Phase 4**: Power features (rules, local search index, OAuth2, multiple accounts)

## Building

Requ1ires Rust and sys1tem dependencies for libcosmic (Wayland dev libraries).

```sh
cargo build
```

## License

Apache-2.0
