# 📚 Typlet documentation

| Document | What it covers |
|---|---|
| [User guide](user-guide.md) · [Anleitung](user-guide.de.md) | Using Typlet: first start, snippets, quick search, forms, diagnostics |
| [Architecture](architecture.md) | Components, process model, expansion pipeline, platform backends, security model |
| [Design](design.md) | Colours, spacing, voice and the English–German glossary |
| [Snippet format](snippet-format.md) | The JSON file format of snippets, with [tested examples](examples/) |
| [Settings file](settings-format.md) | `settings.json`, app rules and the built-in rules |
| [Roadmap](roadmap.md) | Milestones towards V1 and what comes after |
| [Development](development.md) | Setting up, building, testing and project conventions |
| [Test builds](testing/test-build.md) | Trying unsigned builds from CI on Windows and Linux |
| [Manual test checklist](testing/manual-test-checklist.md) | What to test by hand before a release |
| [Decision records](adr/README.md) | Why things are the way they are |

## Keeping the docs current

Documentation is part of every change, not an afterthought. A pull request is complete only when:

- the [feature table](../README.md#-features) and the [roadmap](roadmap.md) reflect the new state,
- [`CHANGELOG.md`](../CHANGELOG.md) has an entry under **Unreleased**,
- architectural decisions are recorded as a new [ADR](adr/README.md),
- format changes are reflected in [snippet-format.md](snippet-format.md) and its examples (CI loads every example),
- visible interface changes come with regenerated [screenshots](images/) and an updated [user guide](user-guide.md),
- every new interface text has a German translation (CI checks this).
