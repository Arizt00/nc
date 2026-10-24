# Reporting security issues

Bootstrap releases are experimental. Manual/raw semantics place memory management under program control and do not guarantee protection against invalid accesses.

For a compiler, loader or distribution issue that may affect other users, retain the package digest and prepare a minimal reproduction. Do not publish credentials, personal information or payloads against third-party services.

Use GitHub's private Report a vulnerability option when enabled. If unavailable, open an issue requesting a private channel without exploit details. Response times are not guaranteed during bootstrap.

Published SHA-256 checksums detect changes relative to the announced artifact. Integrity checksums do not replace cryptographic signatures or a compiler audit.
