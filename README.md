# muskitty-dom

[English](README.md) | [简体中文](README.zh-CN.md)

[![crates.io](https://img.shields.io/crates/v/muskitty-dom.svg)](https://crates.io/crates/muskitty-dom)
[![Documentation](https://docs.rs/muskitty-dom/badge.svg)](https://docs.rs/muskitty-dom)
[![License](https://img.shields.io/crates/l/muskitty-dom.svg)](https://github.com/muskitty-dev/muskitty-dom/blob/main/LICENSE)

DOM core types for the [MusKitty](https://github.com/muskitty-dev) browser engine. A from-scratch, zero-dependency Rust implementation of the DOM Core types and tree operations per the [DOM Standard](https://dom.spec.whatwg.org/).

## Status

- Zero `unsafe` code
- Zero C/C++ dependencies
- Rust stable toolchain only
- 39 unit tests passing

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
muskitty-dom = "0.1.0"
```

Or run:

```bash
cargo add muskitty-dom
```

## Quick Start

```rust
use muskitty_dom::{Node, NodeType};

// Create a Document
let doc = Node::new_document();

// Create an Element and append it
let elem = Node::new_element_html("div", Vec::new(), &doc);
muskitty_dom::append_child(&doc, elem);
```

## What's Implemented

### Node Types

- `Document` — root of the DOM tree
- `DocumentType` — `<!DOCTYPE ...>` declarations
- `DocumentFragment` — lightweight document container (used by `<template>`)
- `Element` — HTML/SVG/MathML elements with attributes
- `Text` — text content
- `Comment` — `<!-- ... -->`
- `ProcessingInstruction` — `<?target data?>`

### Tree Operations

- `append_child` — append a node as the last child
- `insert_before` — insert a node before a reference node
- `remove_child` — remove a child node
- Tree traversal: `parent_node`, `children`, `previous_sibling`, `next_sibling`, `descendants`

### Namespace Support

- HTML, SVG, and MathML namespaces
- Foreign attribute adjustment (xlink, xml, xmlns)
- Template element `content` DocumentFragment

## Architecture

```
muskitty-dom/
  src/
    node.rs                Core Node type with Rc<RefCell> interiority
    document.rs            Document node
    document_type.rs       DocumentType node
    document_fragment.rs   DocumentFragment node
    element.rs             Element node with attributes
    text.rs                Text node
    comment.rs             Comment node
    processing_instruction.rs  ProcessingInstruction node
    attribute.rs           Attribute type with namespace handling
    tree.rs                Tree mutation operations (append/insert/remove)
    error.rs               DOM error types
    lib.rs                 Public API re-exports
```

## Design Principles

1. **DOM Standard is ground truth** — Implementation follows the [DOM Standard](https://dom.spec.whatwg.org/) exactly.
2. **Zero dependencies** — Pure Rust standard library only.
3. **Zero unsafe** — Pure safe Rust.
4. **Interior mutability** — Uses `Rc<RefCell<Node>>` for shared ownership with mutation.

## Testing

```bash
cargo test
```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

Copyright 2026 MusCat / MusKitty Bit-Torch Community
