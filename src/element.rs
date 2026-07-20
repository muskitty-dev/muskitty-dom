//! Element 节点数据。
//!
//! 参见 DOM Living Standard §6 (Elements)。

use std::cell::RefCell;
use std::rc::Rc;

use crate::attribute::{Attribute, Namespace};
use crate::node::Node;

/// `Element` 节点的数据载体。
#[derive(Debug, Clone)]
pub struct ElementData {
    /// 本地名（HTML namespace 下为小写，如 `"div"`）。
    pub local_name: String,
    /// 命名空间分类。
    pub namespace: Namespace,
    /// 命名空间 URI（与 `namespace` 对应）。
    pub namespace_uri: Option<String>,
    /// 限定名前缀（如 `svg:rect` 的 `svg`），HTML namespace 下通常为 `None`。
    pub prefix: Option<String>,
    /// 属性列表（按文档顺序）。
    pub attributes: Vec<Attribute>,
    /// `<template>` 元素的 content DocumentFragment（仅 template 元素非 None）。
    ///
    /// 参见 WHATWG HTML §13.2.6.2：创建 template 元素时同时创建一个
    /// DocumentFragment 作为其 template content。所有插入到 template
    /// 的节点都挂在该 content 下，而非 template 元素本身。
    pub template_content: Option<Rc<RefCell<Node>>>,
    /// `<option>` 元素的 selectedness 状态（WHATWG §4.10.10）。
    /// 初始为 false；若 option 创建时有 `selected` 属性则为 true。
    /// 由 selectedness setting algorithm 维护。仅对 `<option>` 有意义。
    pub selectedness: bool,
}

impl ElementData {
    /// 创建 HTML namespace 下的元素。
    pub fn new_html(local_name: &str, attributes: Vec<Attribute>) -> Self {
        Self {
            local_name: local_name.to_ascii_lowercase(),
            namespace: Namespace::Html,
            namespace_uri: Namespace::Html.uri().map(String::from),
            prefix: None,
            attributes,
            template_content: None,
            selectedness: false,
        }
    }

    /// 创建指定 namespace 下的元素。
    pub fn with_namespace(
        local_name: String,
        namespace: Namespace,
        prefix: Option<String>,
        attributes: Vec<Attribute>,
    ) -> Self {
        Self {
            local_name,
            namespace_uri: namespace.uri().map(String::from),
            namespace,
            prefix,
            attributes,
            template_content: None,
            selectedness: false,
        }
    }

    /// 返回元素的 `node_name`（HTML namespace 下为大写，否则原样）。
    /// 用于 Node.node_name。
    pub fn node_name(&self) -> String {
        match self.namespace {
            Namespace::Html => self.local_name.to_ascii_uppercase(),
            _ => self.local_name.clone(),
        }
    }

    /// 查找指定 local_name 的属性值。
    ///
    /// HTML namespace 下大小写不敏感（DOM §6.1）；SVG/MathML namespace 下
    /// 精确匹配。namespace 通过 `self.namespace` 自动判断。
    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| match self.namespace {
                Namespace::Html => a.local_name.eq_ignore_ascii_case(name),
                _ => a.local_name == name,
            })
            .map(|a| a.value.as_str())
    }

    /// 设置 template content DocumentFragment（仅 template 元素应调用）。
    pub fn set_template_content(&mut self, content: Rc<RefCell<Node>>) {
        self.template_content = Some(content);
    }

    /// `Element.setAttribute(name, value)` — 参见 DOM §6.4。
    ///
    /// 若 `name` 对应的属性已存在则更新其值；否则追加新属性。
    /// HTML namespace 下 name 大小写不敏感。
    pub fn set_attribute(&mut self, name: &str, value: &str) {
        let cmp: fn(&Attribute, &str) -> bool = match self.namespace {
            Namespace::Html => |a, n| a.local_name.eq_ignore_ascii_case(n),
            _ => |a, n| a.local_name == n,
        };
        if let Some(attr) = self.attributes.iter_mut().find(|a| cmp(a, name)) {
            attr.value = value.to_string();
        } else {
            self.attributes.push(Attribute::new(name, value));
        }
    }

    /// `Element.removeAttribute(name)` — 参见 DOM §6.4。
    pub fn remove_attribute(&mut self, name: &str) {
        let idx = match self.namespace {
            Namespace::Html => self
                .attributes
                .iter()
                .position(|a| a.local_name.eq_ignore_ascii_case(name)),
            _ => self.attributes.iter().position(|a| a.local_name == name),
        };
        if let Some(i) = idx {
            self.attributes.remove(i);
        }
    }

    /// `Element.hasAttribute(name)` — 参见 DOM §6.4。
    pub fn has_attribute(&self, name: &str) -> bool {
        self.get_attribute(name).is_some()
    }
}
