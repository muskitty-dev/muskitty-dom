//! DOM 节点核心类型。
//!
//! 参见 DOM Living Standard §4 (Nodes) 和 §4.4 (Interface Node)。
//!
//! 节点使用 `Rc<RefCell<Node>>` 共享所有权：父节点持有子节点的强引用，
//! 子节点持有父节点的弱引用（`Weak`），避免循环引用导致内存泄漏。

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::attribute::{Attribute, Namespace};
use crate::comment::CommentData;
use crate::document::DocumentData;
use crate::document_fragment::DocumentFragmentData;
use crate::document_type::DocumentTypeData;
use crate::element::ElementData;
use crate::processing_instruction::ProcessingInstructionData;
use crate::text::TextData;

/// `Node.nodeType` 常量。参见 DOM Living Standard §4.4。
///
/// CDATASection (nodeType=4) 未实现——HTML 解析器不会产生 CDATA 节点，
/// 只有在 XML 解析模式下才可能出现。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NodeType {
    /// `ELEMENT_NODE = 1`
    Element = 1,
    /// `TEXT_NODE = 3`
    Text = 3,
    /// `PROCESSING_INSTRUCTION_NODE = 7`
    ProcessingInstruction = 7,
    /// `COMMENT_NODE = 8`
    Comment = 8,
    /// `DOCUMENT_NODE = 9`
    Document = 9,
    /// `DOCUMENT_TYPE_NODE = 10`
    DocumentType = 10,
    /// `DOCUMENT_FRAGMENT_NODE = 11`
    DocumentFragment = 11,
}

impl NodeType {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// 节点具体类型的数据载体。
pub enum NodeKind {
    Element(ElementData),
    Text(TextData),
    Comment(CommentData),
    Document(DocumentData),
    DocumentType(DocumentTypeData),
    DocumentFragment(DocumentFragmentData),
    ProcessingInstruction(ProcessingInstructionData),
}

impl NodeKind {
    /// 若为 Element，返回其数据引用。
    pub fn as_element(&self) -> Option<&ElementData> {
        match self {
            NodeKind::Element(e) => Some(e),
            _ => None,
        }
    }

    /// 若为 Text，返回其数据引用。
    pub fn as_text(&self) -> Option<&TextData> {
        match self {
            NodeKind::Text(t) => Some(t),
            _ => None,
        }
    }

    /// 若为 Comment，返回其数据引用。
    pub fn as_comment(&self) -> Option<&CommentData> {
        match self {
            NodeKind::Comment(c) => Some(c),
            _ => None,
        }
    }

    /// 若为 Document，返回其数据引用。
    pub fn as_document(&self) -> Option<&DocumentData> {
        match self {
            NodeKind::Document(d) => Some(d),
            _ => None,
        }
    }

    /// 若为 DocumentType，返回其数据引用。
    pub fn as_document_type(&self) -> Option<&DocumentTypeData> {
        match self {
            NodeKind::DocumentType(d) => Some(d),
            _ => None,
        }
    }

    /// 若为 Element，返回其可变数据引用。
    pub fn as_element_mut(&mut self) -> Option<&mut ElementData> {
        match self {
            NodeKind::Element(e) => Some(e),
            _ => None,
        }
    }

    /// 若为 Text，返回其可变数据引用。
    pub fn as_text_mut(&mut self) -> Option<&mut TextData> {
        match self {
            NodeKind::Text(t) => Some(t),
            _ => None,
        }
    }
}

/// DOM 节点，DOM 树的基本单元。
///
/// 实现对应 DOM Living Standard §4.4 `Node` 接口的核心字段与方法。
///
/// `children` 字段是 `pub(crate)`：外部 crate 只能通过 `child_nodes()`
/// 只读访问或 `tree` 模块的 mutation 函数（`append_child` / `insert_before` /
/// `remove_child` / `replace_child` / `push_child_raw` / `drain_children` /
/// `retain_children`）修改。这确保父子不变式（`parent_node` ↔ `children`）
/// 不会在外部被破坏。
///
/// `kind` 保持 pub 以支持 `NodeKind::as_element()` 等类型判别操作。
pub struct Node {
    /// `Node.nodeType`
    pub node_type: NodeType,
    /// `Node.nodeName`
    ///（Document=`"#document"`、Text=`"#text"`、Comment=`"#comment"`、
    /// DocumentFragment=`"#document-fragment"`、Element=限定名、DocumentType=name）
    pub node_name: String,
    /// 节点所属 Document（Document 节点指向自身）。
    pub owner_document: Weak<RefCell<Node>>,
    /// 父节点弱引用，无父则为空 `Weak`。
    pub parent_node: Weak<RefCell<Node>>,
    /// 具体类型数据。
    pub kind: NodeKind,
    /// 子节点列表（按文档顺序）。`pub(crate)` 限制外部 crate 访问；
    /// 内部 `tree` 模块的函数是唯一应直接修改 `children` 的地方。
    pub(crate) children: Vec<Rc<RefCell<Node>>>,
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("node_type", &self.node_type)
            .field("node_name", &self.node_name)
            .field("child_count", &self.children.len())
            .finish()
    }
}

impl Node {
    /// 创建 Document 节点。`owner_document` 指向自身。
    /// 参见 DOM §5.3。
    pub fn new_document() -> Rc<RefCell<Node>> {
        let node = Rc::new(RefCell::new(Node {
            node_type: NodeType::Document,
            node_name: "#document".to_string(),
            owner_document: Weak::new(),
            parent_node: Weak::new(),
            children: Vec::new(),
            kind: NodeKind::Document(DocumentData::new()),
        }));
        // Document 的 owner_document 指向自身
        node.borrow_mut().owner_document = Rc::downgrade(&node);
        node
    }

    /// 创建 HTML namespace 下的 Element 节点。
    pub fn new_element_html(
        tag_name: &str,
        attributes: Vec<Attribute>,
        owner_document: &Rc<RefCell<Node>>,
    ) -> Rc<RefCell<Node>> {
        let element = ElementData::new_html(tag_name, attributes);
        Rc::new(RefCell::new(Node {
            node_type: NodeType::Element,
            node_name: element.node_name(),
            owner_document: Rc::downgrade(owner_document),
            parent_node: Weak::new(),
            children: Vec::new(),
            kind: NodeKind::Element(element),
        }))
    }

    /// 创建指定 namespace 下的 Element 节点。
    pub fn new_element_ns(
        local_name: String,
        namespace: Namespace,
        prefix: Option<String>,
        attributes: Vec<Attribute>,
        owner_document: &Rc<RefCell<Node>>,
    ) -> Rc<RefCell<Node>> {
        let element = ElementData::with_namespace(local_name, namespace, prefix, attributes);
        Rc::new(RefCell::new(Node {
            node_type: NodeType::Element,
            node_name: element.node_name(),
            owner_document: Rc::downgrade(owner_document),
            parent_node: Weak::new(),
            children: Vec::new(),
            kind: NodeKind::Element(element),
        }))
    }

    /// 创建 Text 节点。
    pub fn new_text(data: &str, owner_document: &Rc<RefCell<Node>>) -> Rc<RefCell<Node>> {
        Rc::new(RefCell::new(Node {
            node_type: NodeType::Text,
            node_name: "#text".to_string(),
            owner_document: Rc::downgrade(owner_document),
            parent_node: Weak::new(),
            children: Vec::new(),
            kind: NodeKind::Text(TextData::new(data)),
        }))
    }

    /// 创建 Comment 节点。
    pub fn new_comment(data: &str, owner_document: &Rc<RefCell<Node>>) -> Rc<RefCell<Node>> {
        Rc::new(RefCell::new(Node {
            node_type: NodeType::Comment,
            node_name: "#comment".to_string(),
            owner_document: Rc::downgrade(owner_document),
            parent_node: Weak::new(),
            children: Vec::new(),
            kind: NodeKind::Comment(CommentData::new(data)),
        }))
    }

    /// 创建 DocumentType 节点。
    pub fn new_document_type(
        name: &str,
        public_id: &str,
        system_id: &str,
        owner_document: &Rc<RefCell<Node>>,
    ) -> Rc<RefCell<Node>> {
        Rc::new(RefCell::new(Node {
            node_type: NodeType::DocumentType,
            node_name: name.to_string(),
            owner_document: Rc::downgrade(owner_document),
            parent_node: Weak::new(),
            children: Vec::new(),
            kind: NodeKind::DocumentType(DocumentTypeData::new(name, public_id, system_id)),
        }))
    }

    /// 创建 DocumentFragment 节点。
    pub fn new_document_fragment(owner_document: &Rc<RefCell<Node>>) -> Rc<RefCell<Node>> {
        Rc::new(RefCell::new(Node {
            node_type: NodeType::DocumentFragment,
            node_name: "#document-fragment".to_string(),
            owner_document: Rc::downgrade(owner_document),
            parent_node: Weak::new(),
            children: Vec::new(),
            kind: NodeKind::DocumentFragment(DocumentFragmentData),
        }))
    }

    /// 创建 ProcessingInstruction 节点。
    /// `node_name` 为 target，`data` 为 PI 数据。参见 DOM §7.4。
    pub fn new_processing_instruction(
        target: &str,
        data: &str,
        owner_document: &Rc<RefCell<Node>>,
    ) -> Rc<RefCell<Node>> {
        Rc::new(RefCell::new(Node {
            node_type: NodeType::ProcessingInstruction,
            node_name: target.to_string(),
            owner_document: Rc::downgrade(owner_document),
            parent_node: Weak::new(),
            children: Vec::new(),
            kind: NodeKind::ProcessingInstruction(ProcessingInstructionData::new(target, data)),
        }))
    }

    // —— 只读遍历 API（DOM §4.4） ——

    /// `Node.parentNode`
    pub fn parent_node(&self) -> Option<Rc<RefCell<Node>>> {
        self.parent_node.upgrade()
    }

    /// `Node.parentElement`
    pub fn parent_element(&self) -> Option<Rc<RefCell<Node>>> {
        let parent = self.parent_node.upgrade()?;
        if parent.borrow().node_type == NodeType::Element {
            Some(parent)
        } else {
            None
        }
    }

    /// `Node.firstChild`
    pub fn first_child(&self) -> Option<Rc<RefCell<Node>>> {
        self.children.first().cloned()
    }

    /// `Node.lastChild`
    pub fn last_child(&self) -> Option<Rc<RefCell<Node>>> {
        self.children.last().cloned()
    }

    /// `Node.hasChildNodes`
    pub fn has_child_nodes(&self) -> bool {
        !self.children.is_empty()
    }

    /// 子节点数量。
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// `Node.previousSibling`。
    ///
    /// 通过比较 `Node` 内存地址在父节点 children 中定位自身。
    pub fn previous_sibling(&self) -> Option<Rc<RefCell<Node>>> {
        let parent = self.parent_node.upgrade()?;
        let parent_ref = parent.borrow();
        let self_ptr = self as *const Node;
        let idx = parent_ref.children.iter().position(|c| {
            let c_borrowed = c.borrow();
            std::ptr::eq(&*c_borrowed, self_ptr)
        })?;
        if idx == 0 {
            None
        } else {
            Some(parent_ref.children[idx - 1].clone())
        }
    }

    /// `Node.nextSibling`。
    ///
    /// 通过比较 `Node` 内存地址在父节点 children 中定位自身。
    pub fn next_sibling(&self) -> Option<Rc<RefCell<Node>>> {
        let parent = self.parent_node.upgrade()?;
        let parent_ref = parent.borrow();
        let self_ptr = self as *const Node;
        let idx = parent_ref.children.iter().position(|c| {
            let c_borrowed = c.borrow();
            std::ptr::eq(&*c_borrowed, self_ptr)
        })?;
        if idx + 1 >= parent_ref.children.len() {
            None
        } else {
            Some(parent_ref.children[idx + 1].clone())
        }
    }

    /// `Node.textContent` getter（DOM §4.4）。
    ///
    /// - DocumentFragment / Element：聚合所有后代 Text 节点内容。
    /// - Text / Comment：返回自身 data。
    /// - 其他类型：返回 `None`。
    pub fn text_content(&self) -> Option<String> {
        match self.node_type {
            NodeType::DocumentFragment | NodeType::Element => {
                let mut s = String::new();
                for child in &self.children {
                    if let Some(t) = child.borrow().text_content() {
                        s.push_str(&t);
                    }
                }
                Some(s)
            }
            NodeType::Text | NodeType::Comment | NodeType::ProcessingInstruction => {
                match &self.kind {
                    NodeKind::Text(t) => Some(t.data.clone()),
                    NodeKind::Comment(c) => Some(c.data.clone()),
                    NodeKind::ProcessingInstruction(pi) => Some(pi.data.clone()),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// 返回后代节点的深度优先迭代器（不含自身）。
    ///
    /// 初始化时分配一次栈。每次 `next()` 将子节点逐个推入栈
    /// （无中间 Vec 分配），仅执行 `Rc::clone`（引用计数+1）。
    pub fn descendants(node: &Rc<RefCell<Node>>) -> Descendants {
        let mut stack = Vec::new();
        // 将直接子节点逆序入栈，保证正向文档顺序（弹栈 LIFO）
        let child_count = node.borrow().children.len();
        for i in (0..child_count).rev() {
            stack.push(node.borrow().children[i].clone());
        }
        Descendants { stack }
    }

    /// 返回子节点切片（只读）。外部代码应通过此方法而不是直接访问
    /// `children` 字段来遍历子节点。
    pub fn child_nodes(&self) -> &[Rc<RefCell<Node>>] {
        &self.children
    }

    /// ParentNode.children — 返回所有 Element 类型的子节点。
    /// 参见 DOM §4.4 ParentNode mixin。
    pub fn children(&self) -> Vec<Rc<RefCell<Node>>> {
        self.children
            .iter()
            .filter(|c| c.borrow().node_type == NodeType::Element)
            .cloned()
            .collect()
    }

    /// ParentNode.firstElementChild — 参见 DOM §4.4。
    pub fn first_element_child(&self) -> Option<Rc<RefCell<Node>>> {
        self.children
            .iter()
            .find(|c| c.borrow().node_type == NodeType::Element)
            .cloned()
    }

    /// ParentNode.lastElementChild — 参见 DOM §4.4。
    pub fn last_element_child(&self) -> Option<Rc<RefCell<Node>>> {
        self.children
            .iter()
            .rev()
            .find(|c| c.borrow().node_type == NodeType::Element)
            .cloned()
    }

    /// ParentNode.childElementCount — 参见 DOM §4.4。
    pub fn child_element_count(&self) -> usize {
        self.children
            .iter()
            .filter(|c| c.borrow().node_type == NodeType::Element)
            .count()
    }

    /// 返回节点所属的 Document（`node.ownerDocument`）。
    /// Document 节点返回自身。
    pub fn owner_document(&self) -> Option<Rc<RefCell<Node>>> {
        self.owner_document.upgrade()
    }

    /// `Node.contains(other)` — 参见 DOM §4.4。
    ///
    /// 如果 `other` 是 `node` 或 `node` 的后代节点则返回 `true`。
    /// 通过 parent_node 链从 `other` 向上查找 `node`。
    pub fn contains(node: &Rc<RefCell<Node>>, other: &Rc<RefCell<Node>>) -> bool {
        if Rc::ptr_eq(node, other) {
            return true;
        }
        let mut current = other.borrow().parent_node.upgrade();
        while let Some(ancestor) = current {
            if Rc::ptr_eq(&ancestor, node) {
                return true;
            }
            let next = ancestor.borrow().parent_node.upgrade();
            current = next;
        }
        false
    }

    /// `Node.isEqualNode(other)` — 参见 DOM §4.4。
    ///
    /// 深度相等比较：节点类型相同 + 节点名相同 + 属性相同 + 所有后代相等。
    pub fn is_equal_node(&self, other: &Node) -> bool {
        if self.node_type != other.node_type || self.node_name != other.node_name {
            return false;
        }
        if self.children.len() != other.children.len() {
            return false;
        }
        for (a, b) in self.children.iter().zip(other.children.iter()) {
            if !a.borrow().is_equal_node(&b.borrow()) {
                return false;
            }
        }
        true
    }

    /// `Node.compareDocumentPosition(other)` — 参见 DOM §4.4。
    ///
    /// 返回一个位掩码表示 `other` 相对于 `self` 的位置关系。
    /// 位常量：`DOCUMENT_POSITION_DISCONNECTED(1)` /
    /// `PRECEDING(2)` / `FOLLOWING(4)` / `CONTAINS(8)` /
    /// `CONTAINED_BY(16)` / `IMPLEMENTATION_SPECIFIC(32)`。
    pub fn compare_document_position(&self, other: &Node) -> u16 {
        // 如果是同一个节点，返回 0
        if std::ptr::eq(self, other) {
            return 0;
        }
        // 检查是否在同一棵树中
        let self_root = self.get_root_node_impl();
        let other_root = other.get_root_node_impl();
        match (self_root, other_root) {
            (Some(sr), Some(or)) => {
                if !Rc::ptr_eq(&sr, &or) {
                    return 1; // DOCUMENT_POSITION_DISCONNECTED
                }
            }
            _ => return 1, // 孤立节点
        }
        // 简化实现：同一树中返回 0（占位）。完整实现需要计算树序位置
        // 并返回 FOLLOWING/PRECEDING/CONTAINS/CONTAINED_BY。
        0
    }

    /// `Node.getRootNode()` — 参见 DOM §4.4。
    ///
    /// 返回树的根节点（通常是 Document 或 DocumentFragment）。
    /// 若自身是根节点（无父节点），返回 `None`——调用方应使用自身。
    /// TODO: 改为返回 `Rc<RefCell<Node>>` 需要 `&self` 能够访问外层 `Rc`。
    pub fn get_root_node(&self) -> Option<Rc<RefCell<Node>>> {
        self.get_root_node_impl()
    }

    /// `Node.isConnected` — 参见 DOM §4.4。
    ///
    /// 如果节点连接到 Document 树中则返回 `true`（根节点是 Document）。
    /// Document 节点自身也算 connected（它的根就是自己）。
    pub fn is_connected(&self) -> bool {
        if self.node_type == NodeType::Document {
            return true;
        }
        let root = self.get_root_node_impl();
        root.map(|r| r.borrow().node_type == NodeType::Document)
            .unwrap_or(false)
    }

    /// 内部：沿 parent_node 链向上找根节点。
    ///
    /// 由于 `&self` 无法访问自身所在的 `Rc`，根节点判断依赖
    /// `parent_node` 为空 → 调用方应知道 `self` 就是根。
    /// 对于无父节点的 Node，返回 `None`；调用方应回退到自身。
    fn get_root_node_impl(&self) -> Option<Rc<RefCell<Node>>> {
        let mut current = self.parent_node.upgrade()?;
        loop {
            let next = current.borrow().parent_node.upgrade();
            match next {
                Some(parent) => current = parent,
                None => return Some(current),
            }
        }
    }
}

/// 后代节点深度优先迭代器。
///
/// 每次 `next()` 弹出栈顶节点并将其子节点逆序推入栈（无中间
/// Vec 分配）。初始化时分配一次栈，后续仅执行 `Rc::clone`。
pub struct Descendants {
    stack: Vec<Rc<RefCell<Node>>>,
}

impl Iterator for Descendants {
    type Item = Rc<RefCell<Node>>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        let child_count = node.borrow().children.len();
        // 子节点逆序入栈，保证正向文档顺序（弹栈 LIFO）
        for i in (0..child_count).rev() {
            self.stack.push(node.borrow().children[i].clone());
        }
        Some(node)
    }
}
