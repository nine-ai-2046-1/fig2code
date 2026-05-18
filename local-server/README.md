local-server

Usage:

    cargo run -- http 8012 static ./test1

This starts a simple HTTP static file server bound to localhost:8012 serving files from ./test1.

Security rules enforced:
- Binds only to 127.0.0.1 (localhost) to avoid exposing to the internet.
- Disallows directory listing (returns 403 if no index.html present).
- Prevents path traversal: requests containing ".." or which resolve outside the root are rejected.
