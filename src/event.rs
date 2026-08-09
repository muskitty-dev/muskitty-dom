//! DOM 事件模型（DOM Living Standard §4.4 / §4.5 / §4.6）。
//!
//! 提供 `Event`、`EventTarget` 语义的最小实现：
//! `add_event_listener` / `remove_event_listener` / `dispatch_event`。
//!
//! 监听器以 `Rc<dyn Fn(&Event)>` 存储：`Rc` 提供指针身份，使
//! `removeEventListener` 能按同一回调实例精确删除（DOM §4.4
//! "removeEventListener" 要求比较 callback 的 identity）。
//!
//! 派发算法（§4.6 "To dispatch an event"）在无 Shadow DOM / 无
//! relatedTarget 的简化前提下实现三阶段：捕获（祖先自上而下）、
//! target、冒泡（祖先自下而上）。所有派发期间的树/监听器访问都以
//! 独立 borrow 完成，绝不跨回调持有 `Ref`，因此监听器内可安全地
//! 增删监听器或再次派发。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::node::Node;

/// 监听器回调类型。`Rc` 提供指针身份，供 removeEventListener 精确定位。
pub type EventCallback = Rc<dyn Fn(&Event)>;

/// `Event.eventPhase` 常量。参见 DOM §4.5。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPhase {
    /// `NONE = 0`：未在派发中。
    None,
    /// `CAPTURING_PHASE = 1`
    Capturing,
    /// `AT_TARGET = 2`
    AtTarget,
    /// `BUBBLING_PHASE = 3`
    Bubbling,
}

impl EventPhase {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// `addEventListener` 选项子集。参见 DOM §4.4。
#[derive(Debug, Clone, Copy, Default)]
pub struct EventListenerOptions {
    /// `capture`：捕获阶段调用。
    pub capture: bool,
    /// `once`：调用一次后自动移除。
    pub once: bool,
}

/// 存储在 `Node.event_listeners` 中的单条监听器记录。
pub(crate) struct EventListenerEntry {
    /// 监听的事件类型（如 `"click"`）。
    pub type_: String,
    /// 是否捕获阶段监听。
    pub capture: bool,
    /// 是否一次性监听（触发后移除）。
    pub once: bool,
    /// 惰性删除标记：派发中移除时置位，避免派发中修改 Vec 引发借用冲突。
    pub removed: bool,
    /// 回调。`Rc` 提供指针身份，供 removeEventListener 精确定位。
    pub callback: EventCallback,
}

/// `Event` 对象。参见 DOM §4.5。
///
/// 只读标志（stop propagation / defaultPrevented）用 `Cell<bool>` 存储，
/// 使监听器从 `&Event` 即可修改派发状态——这是零依赖 crate 下的
/// 内部可变性方案，无需 `&mut Event`。
pub struct Event {
    type_: String,
    /// 是否冒泡（DOM §4.5 `bubbles`）。构造时决定。
    pub bubbles: bool,
    /// 是否可取消（DOM §4.5 `cancelable`）。构造时决定。
    pub cancelable: bool,
    /// 派发目标（§4.6 派发时设置）。
    target: Option<Rc<RefCell<Node>>>,
    /// 当前正在调用监听器的节点（§4.5 `currentTarget`）。
    current_target: Option<Rc<RefCell<Node>>>,
    /// 当前派发阶段（§4.5 `eventPhase`）。
    event_phase: EventPhase,
    propagation_stopped: Cell<bool>,
    immediate_propagation_stopped: Cell<bool>,
    default_prevented: Cell<bool>,
}

impl Event {
    /// 创建事件对象。参见 DOM §4.5 `Event(type, eventInitDict)`。
    pub fn new(type_: &str, bubbles: bool, cancelable: bool) -> Self {
        Event {
            type_: type_.to_string(),
            bubbles,
            cancelable,
            target: None,
            current_target: None,
            event_phase: EventPhase::None,
            propagation_stopped: Cell::new(false),
            immediate_propagation_stopped: Cell::new(false),
            default_prevented: Cell::new(false),
        }
    }

    /// `Event.type`。
    pub fn type_(&self) -> &str {
        &self.type_
    }

    /// `Event.target`。派发中可见，否则 `None`。
    pub fn target(&self) -> Option<Rc<RefCell<Node>>> {
        self.target.clone()
    }

    /// `Event.currentTarget`。调用某节点监听器期间为该节点。
    pub fn current_target(&self) -> Option<Rc<RefCell<Node>>> {
        self.current_target.clone()
    }

    /// `Event.eventPhase`。
    pub fn event_phase(&self) -> EventPhase {
        self.event_phase
    }

    /// `Event.stopPropagation()`。停止后续节点（捕获剩余 + target + 冒泡）。
    pub fn stop_propagation(&self) {
        self.propagation_stopped.set(true);
    }

    /// `Event.stopImmediatePropagation()`。
    /// 停止当前节点剩余监听器及所有后续节点。
    pub fn stop_immediate_propagation(&self) {
        self.propagation_stopped.set(true);
        self.immediate_propagation_stopped.set(true);
    }

    /// `Event.preventDefault()`。仅 `cancelable` 为 true 时生效。
    pub fn prevent_default(&self) {
        if self.cancelable {
            self.default_prevented.set(true);
        }
    }

    /// `Event.defaultPrevented`。
    pub fn default_prevented(&self) -> bool {
        self.default_prevented.get()
    }

    /// 内部：监听器是否可见 `stopPropagation` 标志。
    fn propagation_stopped(&self) -> bool {
        self.propagation_stopped.get()
    }

    /// 内部：监听器是否可见 `stopImmediatePropagation` 标志。
    fn immediate_propagation_stopped(&self) -> bool {
        self.immediate_propagation_stopped.get()
    }
}

/// `EventTarget.addEventListener(type, callback, options)`。参见 DOM §4.4。
pub fn add_event_listener(
    target: &Rc<RefCell<Node>>,
    type_: &str,
    callback: EventCallback,
    options: EventListenerOptions,
) {
    target
        .borrow_mut()
        .event_listeners
        .push(EventListenerEntry {
            type_: type_.to_string(),
            capture: options.capture,
            once: options.once,
            removed: false,
            callback,
        });
}

/// `EventTarget.removeEventListener(type, callback, options)`。参见 DOM §4.4。
///
/// 按 (type, capture, callback 指针身份) 精确匹配；命中的条目被标记
/// `removed`（而非物理删除），派发中删除不会破坏正在迭代的列表。
pub fn remove_event_listener(
    target: &Rc<RefCell<Node>>,
    type_: &str,
    callback: &EventCallback,
    capture: bool,
) {
    let mut n = target.borrow_mut();
    for entry in &mut n.event_listeners {
        if !entry.removed
            && entry.type_ == type_
            && entry.capture == capture
            && Rc::ptr_eq(&entry.callback, callback)
        {
            entry.removed = true;
            return;
        }
    }
}

/// `EventTarget.dispatchEvent(event)`。参见 DOM §4.6。
///
/// 返回 `!defaultPrevented`：任一可取消监听器调用了 `preventDefault`
/// 则返回 `false`（对应 spec 的 "canceled" 语义）。
///
/// §4.6 在 path 上做两趟遍历：
/// 1. **reverse pass**（root → target），invocation phase = "capturing"，
///    只调用 `capture: true` 的监听器；target 的捕获监听器在此趟触发，
///    此时 `eventPhase` 为 `AT_TARGET`。
/// 2. **forward pass**（target → root），invocation phase = "bubbling"，
///    只调用 `capture: false` 的监听器；target 的非捕获监听器在此趟触发，
///    `eventPhase` 为 `AT_TARGET`；非 target 祖先的 `eventPhase` 为
///    `BUBBLING_PHASE`，且 `bubbles == false` 时跳过（`continue`）。
///
/// 简化说明：不支持 `relatedTarget`、Shadow DOM（`shadowAdjustedTarget`）、
/// 以及派发中重入同一 `Event` 的保护——重入使用新 `Event` 对象是安全的。
pub fn dispatch_event(target: &Rc<RefCell<Node>>, event: &mut Event) -> bool {
    // §4.6: 设 event.target = target，并复位派发态。
    event.propagation_stopped.set(false);
    event.immediate_propagation_stopped.set(false);
    event.default_prevented.set(false);
    event.event_phase = EventPhase::None;
    event.target = Some(target.clone());

    // §4.6: 沿 parent 链收集 path（自底向上含 target）。
    let mut path: Vec<Rc<RefCell<Node>>> = vec![target.clone()];
    let mut current = target.borrow().parent_node.upgrade();
    while let Some(node) = current {
        path.push(node.clone());
        let next = node.borrow().parent_node.upgrade();
        current = next;
    }

    // §4.6 reverse pass：root → target，调用捕获监听器。
    // 每个 invoke 前若 stop propagation 已置位则整趟跳过（invoke step 6）。
    for node in path.iter().rev() {
        let is_target = Rc::ptr_eq(node, &path[0]);
        event.event_phase = if is_target {
            EventPhase::AtTarget
        } else {
            EventPhase::Capturing
        };
        event.current_target = Some(node.clone());
        dispatch_to_node(node, event, InvocationPhase::Capturing);
        if event.immediate_propagation_stopped() || event.propagation_stopped() {
            break;
        }
    }

    // §4.6 forward pass：target → root，调用非捕获监听器。
    if !event.propagation_stopped() && !event.immediate_propagation_stopped() {
        for node in path.iter() {
            let is_target = Rc::ptr_eq(node, &path[0]);
            if is_target {
                event.event_phase = EventPhase::AtTarget;
            } else {
                if !event.bubbles {
                    continue; // 非冒泡事件跳过祖先（target 仍触发）
                }
                event.event_phase = EventPhase::Bubbling;
            }
            event.current_target = Some(node.clone());
            dispatch_to_node(node, event, InvocationPhase::Bubbling);
            if event.immediate_propagation_stopped() || event.propagation_stopped() {
                break;
            }
        }
    }

    // §4.6 收尾：复位派发态。
    event.current_target = None;
    event.event_phase = EventPhase::None;

    !event.default_prevented()
}

/// 监听器调用阶段（§4.6 "invocation phase"），决定取哪个监听器集合。
enum InvocationPhase {
    /// 捕获：仅 `capture: true` 的监听器。
    Capturing,
    /// 冒泡：仅 `capture: false` 的监听器。
    Bubbling,
}

/// 对单个节点调用匹配的监听器。参见 DOM §4.6 "To invoke"。
///
/// 先在单一 borrow 内快照匹配的 `(once, callback)` 列表（持有 Rc 句柄），
/// 再逐个调用——绝不跨回调持有 `Ref`。调用后再用新 borrow 处理
/// `once` 移除，避免借用冲突。
fn dispatch_to_node(node: &Rc<RefCell<Node>>, event: &mut Event, phase: InvocationPhase) {
    let snapshot: Vec<(bool, EventCallback)> = {
        let n = node.borrow();
        n.event_listeners
            .iter()
            .filter(|entry| {
                !entry.removed
                    && entry.type_ == event.type_
                    && match phase {
                        InvocationPhase::Capturing => entry.capture,
                        InvocationPhase::Bubbling => !entry.capture,
                    }
            })
            .map(|entry| (entry.once, entry.callback.clone()))
            .collect()
    };

    for (once, callback) in snapshot {
        if event.immediate_propagation_stopped() {
            break;
        }
        callback(event);
        // §4.4 once：触发后从真实列表移除（按回调指针身份匹配单条）。
        if once {
            let mut n = node.borrow_mut();
            let mut removed = false;
            n.event_listeners.retain(|entry| {
                if !removed
                    && !entry.removed
                    && entry.once
                    && Rc::ptr_eq(&entry.callback, &callback)
                {
                    removed = true;
                    false
                } else {
                    true
                }
            });
        }
    }
}
