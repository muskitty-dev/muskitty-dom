# muskitty-dom

[English](README.md) | [简体中文](README.zh-CN.md)

[![crates.io](https://img.shields.io/crates/v/muskitty-dom.svg)](https://crates.io/crates/muskitty-dom)
[![Documentation](https://docs.rs/muskitty-dom/badge.svg)](https://docs.rs/muskitty-dom)
[![License](https://img.shields.io/crates/l/muskitty-dom.svg)](https://github.com/muskitty-dev/muskitty-dom/blob/main/LICENSE)

[MusKitty](https://github.com/muskitty-dev) 浏览器引擎的 DOM 核心类型库。一个从零开始、零依赖的 Rust 实现，严格遵循 [DOM Standard](https://dom.spec.whatwg.org/) 中的 DOM Core 类型与树操作规范。

## 状态

- 零 `unsafe` 代码
- 零 C/C++ 依赖
- 仅依赖 Rust 稳定版工具链
- 39 个单元测试通过

## 安装

在你的 `Cargo.toml` 中添加：

```toml
[dependencies]
muskitty-dom = "0.1.0"
```

或运行：

```bash
cargo add muskitty-dom
```

## 快速上手

```rust
use muskitty_dom::{Node, NodeType};

// Create a Document
let doc = Node::new_document();

// Create an Element and append it
let elem = Node::new_element_html("div", Vec::new(), &doc);
muskitty_dom::append_child(&doc, elem);
```

## 已实现的功能

### 节点类型

- `Document` — DOM 树的根节点
- `DocumentType` — `<!DOCTYPE ...>` 声明
- `DocumentFragment` — 轻量级文档容器（用于 `<template>`）
- `Element` — 带属性的 HTML/SVG/MathML 元素
- `Text` — 文本内容
- `Comment` — `<!-- ... -->`
- `ProcessingInstruction` — `<?target data?>`

### 树操作

- `append_child` — 将节点追加为最后一个子节点
- `insert_before` — 在参考节点之前插入一个节点
- `remove_child` — 移除一个子节点
- 树遍历：`parent_node`、`children`、`previous_sibling`、`next_sibling`、`descendants`

### 命名空间支持

- HTML、SVG 和 MathML 命名空间
- 外来属性调整（xlink、xml、xmlns）
- Template 元素的 `content` DocumentFragment

## 架构

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

## 设计原则

1. **以 DOM Standard 为唯一准绳** — 实现严格遵循 [DOM Standard](https://dom.spec.whatwg.org/)。
2. **零依赖** — 仅使用 Rust 标准库。
3. **零 unsafe** — 纯安全 Rust 实现。
4. **内部可变性** — 使用 `Rc<RefCell<Node>>` 实现带可变的共享所有权。

## 测试

```bash
cargo test
```

## 许可证

基于 Apache License, Version 2.0 授权，详见 [LICENSE](LICENSE)。

Copyright 2026 MusCat / MusKitty Bit-Torch Community
