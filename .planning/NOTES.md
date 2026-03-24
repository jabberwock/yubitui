# Notes

- 2026-03-24: Must be cross-platform (Linux/macOS/Windows). No exceptions. All paths, hints, service names must be platform-aware.
- 2026-03-24: Always commit with GPG signing (`git commit -S`).
- 2026-03-24: Research is allowed/encouraged in planning phases.
- 2026-03-24: User has asked multiple times for menus in the UI — dropdown/context menus in the TUI. This is a recurring request that keeps getting missed. Must be addressed in upcoming planning.
- 2026-03-24: User wants wizards for complex tasks — not just menu items that launch gpg blindly.
- 2026-03-24 BUG: SSH detection false negative on Windows — `dirs::home_dir()` may not resolve `~/.gnupg/gpg-agent.conf` correctly on Windows, causing `enable-ssh-support` to always appear missing even when set. Fix: check Windows-specific path `%APPDATA%\gnupg\gpg-agent.conf`.
- 2026-03-24 BUG: PIN unblock wizard is incomplete — `unblock_user_pin()` calls `gpg --card-edit passwd 2` which requires a reset code. If no reset code exists (retries = 0), it fails silently. Need a full wizard: detect reset code availability → if none, offer factory reset path (gpg --card-edit factory-reset) with clear warning that all keys will be wiped.
