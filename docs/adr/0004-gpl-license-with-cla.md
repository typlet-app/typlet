# 0004. GPL-3.0-or-later with a contributor licence agreement

- Status: Accepted
- Date: 2026-10-23

## Context

Typlet is open source and donation-funded; later monetisation (for example a paid Store edition or a hosted team sync) should remain possible. Constraints:

- Slint is free for open-source use under GPL-3.0.
- Free code signing from SignPath Foundation requires an OSI-approved licence and no proprietary components.
- Clones that take the code closed should not be possible.

## Decision

- Typlet is licensed **GPL-3.0-or-later**.
- Contributors sign a [Contributor License Agreement](../../CLA.md) once. They keep their copyright and grant the project the right to relicense, which keeps dual licensing possible. The CLA also promises that contributions stay available under an open-source licence.
- The name and logo are protected separately by a [trademark policy](../../TRADEMARKS.md); forks must use a different name.

## Consequences

- Compatible with Slint (GPL), SignPath and all permissively licensed dependencies (checked by `cargo-deny`).
- Users and companies can use Typlet freely; the GPL only applies when distributing modified versions.
- A proprietary desktop edition would no longer qualify for SignPath and would need its own signing certificate.
- Contributors must sign the CLA before their first pull request is merged.
