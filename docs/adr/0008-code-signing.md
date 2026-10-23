# 0008. Unsigned beta, then free signing via SignPath and the Microsoft Store

- Status: Accepted
- Date: 2026-10-23

## Context

Unsigned Windows programs run fully, but SmartScreen warns on first start, antivirus heuristics are stricter with programs that read the keyboard, and Smart App Control (Windows 11) blocks them entirely. The budget for certificates is zero. Microsoft's low-cost signing service does not accept individual developers in the EU.

## Decision

1. Early test builds are **unsigned**; the README explains the SmartScreen prompt.
2. Once the repository has public releases, the project applies to the **SignPath Foundation** for free open-source code signing. Builds are signed from CI only.
3. Typlet is also submitted to the **Microsoft Store** as MSIX, which Microsoft signs for free.
4. macOS notarisation (paid Apple developer account) is decided when the macOS port starts.

## Consequences

- No functionality depends on signing.
- Release builds must come from CI and be reproducible from the repository (SignPath requirement).
- Repository members must use two-factor authentication.
- Typing into programs that run as administrator requires a signed build with UIAccess; this stays out of scope until signing is in place.
