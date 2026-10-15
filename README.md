# MailDesk — Linux desktop Gmail IMAP client

MailDesk is a Tauri 2 + WebKitGTK desktop-client starter for Linux, targeting Q4OS/Debian and AppImage packaging. The included frontend is an interactive UI prototype; the Rust backend includes initial Gmail IMAP/SMTP and secure-credential primitives. It is **not yet production-ready**: see the limitations below.

## Requirements

- Q4OS / Debian-family Linux with a working WebKitGTK 4.1 runtime
- Node.js 20+ and npm
- Rust stable toolchain and Cargo
- Tauri Linux build dependencies (`build-essential`, `curl`, `wget`, `file`, `libxdo-dev`, `libssl-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `patchelf`, and the WebKitGTK 4.1 development package appropriate to your Q4OS release)
- A Gmail account with IMAP enabled and 2-Step Verification enabled, plus a Google App Password

Check the current Tauri Linux prerequisites for the exact package names supported by your Debian/Q4OS release before installing system packages.

## Run the UI prototype

```bash
npm install
npm run dev
```

This browser preview uses clearly labelled sample messages and browser localStorage for demonstration only. It does not connect to Gmail and does not send email.

## Build the desktop app

```bash
npm install
cd src-tauri
cargo check
cd ..
npm run tauri -- build --bundles appimage
```

The AppImage is normally emitted under `src-tauri/target/release/bundle/appimage/`. The build has not been run in this environment; compile and runtime testing on the target Q4OS laptop is required.

## Gmail setup

1. Enable IMAP for the Gmail account if applicable in Gmail settings.
2. Enable 2-Step Verification and create a dedicated App Password in your Google Account security settings.
3. Enter the email address and App Password in the desktop app's account setup flow. The backend `save_credentials` command stores the secret in the OS credential service via `keyring`; it does not write it to preferences.
4. IMAP endpoint: `imap.gmail.com:993` with TLS. SMTP endpoint: `smtp.gmail.com:465` with TLS.

Never paste an App Password into logs, source code, bug reports, or preference files. Revoke the App Password from Google Account security if it is exposed.

## Privacy model

- Credential storage: OS keyring via the `keyring` crate. Linux availability depends on a working Secret Service/keyring session and may require a desktop keyring to be unlocked.
- Preferences: `preferences.json` under the Tauri application config directory; intended for theme, column layouts, page size, identity selection, bookmarks and pins only.
- Email: do not add persistent message caches, body indexes, attachment files or local draft files. Message content and drafts should remain in memory and be stored remotely only when explicitly sent or saved as a Gmail draft.
- External URLs: treat web content as untrusted. A production implementation should isolate external browsing from mail privileges and must not expose Tauri IPC commands to arbitrary website origins.

## Important implementation gaps before production

- Wire the UI to the Rust commands; the included HTML currently demonstrates interactions with sample data.
- Implement robust MIME parsing, HTML/plain-text alternatives, inline images, attachment upload/download streamed without persistent local copies, quoted replies, threading, draft save/delete on Gmail, labels, archive, spam/trash, read/unread, star, and search.
- Implement real Gmail labels/identity behaviour. Gmail labels are exposed through IMAP in a Gmail-specific way and are not always equivalent to ordinary folders.
- Implement server-side sorting/search and stable pagination. The initial `list_messages` command uses a basic IMAP `SEARCH ALL` and UID ordering, not full server-side column sorting; not every IMAP server supports the `SORT` extension.
- Add account setup and connection-error UI, reconnect handling, rate limits, cancellation, message sanitisation, phishing protections, and automated tests.
- The requested automatic signature import is not supported by App Password + IMAP/SMTP. Gmail signatures and Send mail as identity configuration require Gmail API / OAuth or manual signature configuration in MailDesk. This project intentionally follows Option A and must not claim automatic Gmail signature sync.
- Build and verify the AppImage on the actual Q4OS release. This package is a development starter, not a signed, tested release binary.

## License

MIT for this starter project; review third-party dependency licenses before distribution.
