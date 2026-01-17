# Introduction

This book contains the governance documentation for **govctl**, an opinionated CLI for RFC-driven software development.

## How This Book Is Organized

### Specifications

RFCs (Requests for Comments) are the normative specifications that define govctl's behavior. They are constitutional law — implementation must conform to them.

- **[RFC-0000](./rfc/RFC-0000.md)**: The governance framework itself. Start here to understand the core concepts: RFCs, Clauses, ADRs, and Work Items.
- **[RFC-0001](./rfc/RFC-0001.md)**: Lifecycle state machines for all artifact types.

### Decisions

ADRs (Architectural Decision Records) document significant design choices. They explain *why* things are built a certain way.

### Work Items

Work Items track units of work from inception to completion. They provide an audit trail of what was done and when.

## The Data Model

All governance artifacts have a **Single Source of Truth (SSOT)** in the `gov/` directory:

```
gov/
├── config.toml           # govctl configuration
├── rfc/                  # RFC-NNNN/rfc.json + clauses/
├── adr/                  # ADR-NNNN-*.toml
└── work/                 # WI-YYYY-MM-DD-NNN-*.toml
```

The markdown files in this book are **rendered projections** — generated from the SSOT by `govctl render`. Each includes a SHA-256 signature for tampering detection.

## Phase Discipline

govctl enforces a strict phase lifecycle:

```
spec → impl → test → stable
```

- **spec**: Defining what will be built. No implementation work permitted.
- **impl**: Building what was specified.
- **test**: Verifying implementation matches specification.
- **stable**: Released for production use.

Phases cannot be skipped. This discipline ensures specifications precede implementation.

## Getting Started

1. Read [RFC-0000](./rfc/RFC-0000.md) to understand the governance model
2. Install govctl: `cargo install govctl`
3. Initialize a project: `govctl init`
4. Create your first RFC: `govctl new rfc "Feature Title"`

For CLI usage details, see the [README](https://github.com/govctl-org/govctl#readme).
