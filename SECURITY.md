# 🔒 Security policy

Typlet sees what users type. Security and privacy problems are treated with the highest priority.

## Reporting a vulnerability

**Please do not open a public issue.** Instead:

1. Open the repository's **Security** tab.
2. Choose **Report a vulnerability** to send a private report.

Please include what you found, how to reproduce it and what an attacker could achieve. You will receive an acknowledgement within 7 days. We will keep you informed and credit you in the release notes unless you prefer otherwise.

## Supported versions

Typlet has no stable release yet. Security fixes go into the `main` branch and the next release.

## What counts as a vulnerability

Examples:

- typed text being stored, logged or sent anywhere,
- expansion in password fields,
- a crafted snippet file that executes code, crashes Typlet or leaks data,
- links in snippets that bypass the allowed schemes (`http`, `https`, `mailto`),
- weaknesses in installers, updates or signing.

## Design principles

The binding security rules for the code base are described in the [security and privacy model](docs/architecture.md#security-and-privacy-model).
