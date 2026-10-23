# 📚 Typlet documentation

| Document | What it covers |
|---|---|
| [Architecture](architecture.md) | Components, process model, expansion pipeline, platform backends, security model |
| [Snippet format](snippet-format.md) | The JSON file format of snippets, with [tested examples](examples/) |
| [Roadmap](roadmap.md) | Milestones towards V1 and what comes after |
| [Development](development.md) | Setting up, building, testing and project conventions |
| [Decision records](adr/README.md) | Why things are the way they are |

## Keeping the docs current

Documentation is part of every change, not an afterthought. A pull request is complete only when:

- the [feature table](../README.md#-features) and the [roadmap](roadmap.md) reflect the new state,
- [`CHANGELOG.md`](../CHANGELOG.md) has an entry under **Unreleased**,
- architectural decisions are recorded as a new [ADR](adr/README.md),
- format changes are reflected in [snippet-format.md](snippet-format.md) and its examples (CI loads every example).
