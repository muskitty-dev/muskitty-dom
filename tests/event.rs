//! DOM 事件模型集成测试（DOM §4.4 / §4.5 / §4.6）。
//!
//! 覆盖：基本触发、type/target/currentTarget、捕获与冒泡顺序、
//! stopPropagation / stopImmediatePropagation、once、removeEventListener、
//! 派发中增删监听器、preventDefault 语义。

use std::cell::RefCell;
use std::rc::Rc;

use muskitty_dom::{
    add_event_listener, append_child, dispatch_event, remove_event_listener, Event, EventCallback,
    EventListenerOptions, EventPhase, Node,
};

/// 测试内节点引用别名（clippy type_complexity）。
type NodeRef = Rc<RefCell<Node>>;

/// 构造 3 层元素树：grand → parent → child。
fn tree() -> (NodeRef, NodeRef, NodeRef) {
    let doc = Node::new_document();
    let grand = Node::new_element_html("grand", vec![], &doc);
    let parent = Node::new_element_html("parent", vec![], &doc);
    let child = Node::new_element_html("child", vec![], &doc);
    append_child(&grand, parent.clone()).unwrap();
    append_child(&parent, child.clone()).unwrap();
    (grand, parent, child)
}

/// 只挂 target 层一个元素（无祖先）的树。
fn single() -> NodeRef {
    let doc = Node::new_document();
    Node::new_element_html("solo", vec![], &doc)
}

#[test]
fn basic_trigger() {
    let (_, _, child) = tree();
    let count = Rc::new(RefCell::new(0));
    let c2 = count.clone();
    add_event_listener(
        &child,
        "click",
        Rc::new(move |_| *c2.borrow_mut() += 1),
        EventListenerOptions::default(),
    );
    let mut event = Event::new("click", true, true);
    dispatch_event(&child, &mut event);
    assert_eq!(*count.borrow(), 1, "listener should fire once");
}

#[test]
fn type_and_target_visible() {
    let (_, _, child) = tree();
    let observed_type = Rc::new(RefCell::new(String::new()));
    let observed_target = Rc::new(RefCell::new(0usize));
    let t = observed_type.clone();
    let g = observed_target.clone();
    add_event_listener(
        &child,
        "load",
        Rc::new(move |e| {
            *t.borrow_mut() = e.type_().to_string();
            let target = e.target().expect("target set during dispatch");
            *g.borrow_mut() = Rc::as_ptr(&target) as usize;
        }),
        EventListenerOptions::default(),
    );
    let mut event = Event::new("load", true, true);
    dispatch_event(&child, &mut event);
    assert_eq!(*observed_type.borrow(), "load");
    assert_eq!(
        *observed_target.borrow(),
        Rc::as_ptr(&child) as usize,
        "event.target should be the dispatch target"
    );
}

#[test]
fn bubbling_order_and_phase() {
    let (grand, parent, child) = tree();
    let order = Rc::new(RefCell::new(Vec::new()));
    let phases = Rc::new(RefCell::new(Vec::new()));
    for (node, tag) in [
        (grand.clone(), "grand"),
        (parent.clone(), "parent"),
        (child.clone(), "child"),
    ] {
        let o = order.clone();
        let p = phases.clone();
        add_event_listener(
            &node,
            "click",
            Rc::new(move |e| {
                o.borrow_mut().push(tag.to_string());
                p.borrow_mut().push(e.event_phase());
            }),
            EventListenerOptions::default(),
        );
    }
    let mut event = Event::new("click", true, true);
    dispatch_event(&child, &mut event);
    assert_eq!(
        *order.borrow(),
        vec!["child", "parent", "grand"],
        "bubbling order: target first, then ancestors bottom-up"
    );
    assert_eq!(
        *phases.borrow(),
        vec![
            EventPhase::AtTarget,
            EventPhase::Bubbling,
            EventPhase::Bubbling
        ],
        "target at-target phase, ancestors bubbling phase"
    );
}

#[test]
fn capture_order_top_down() {
    let (grand, parent, child) = tree();
    let order = Rc::new(RefCell::new(Vec::new()));
    let phases = Rc::new(RefCell::new(Vec::new()));
    for (node, tag) in [
        (grand.clone(), "grand"),
        (parent.clone(), "parent"),
        (child.clone(), "child"),
    ] {
        let o = order.clone();
        let p = phases.clone();
        add_event_listener(
            &node,
            "click",
            Rc::new(move |e| {
                o.borrow_mut().push(tag.to_string());
                p.borrow_mut().push(e.event_phase());
            }),
            EventListenerOptions {
                capture: true,
                once: false,
            },
        );
    }
    let mut event = Event::new("click", true, true);
    dispatch_event(&child, &mut event);
    assert_eq!(
        *order.borrow(),
        vec!["grand", "parent", "child"],
        "capture order: root down to target"
    );
    assert_eq!(
        *phases.borrow(),
        vec![
            EventPhase::Capturing,
            EventPhase::Capturing,
            EventPhase::AtTarget
        ],
        "ancestors see capturing, target capture listener sees at-target (§4.6 reverse pass)"
    );
}

#[test]
fn target_capture_and_normal_both_fire() {
    // §4.6: target 先以捕获阶段调用 capture 监听器，再以 at-target 调用全部。
    let (_, _, child) = tree();
    let order = Rc::new(RefCell::new(Vec::new()));
    {
        let o = order.clone();
        add_event_listener(
            &child,
            "click",
            Rc::new(move |_| o.borrow_mut().push("capture".to_string())),
            EventListenerOptions {
                capture: true,
                once: false,
            },
        );
    }
    {
        let o = order.clone();
        add_event_listener(
            &child,
            "click",
            Rc::new(move |_| o.borrow_mut().push("normal".to_string())),
            EventListenerOptions::default(),
        );
    }
    let mut event = Event::new("click", true, true);
    dispatch_event(&child, &mut event);
    assert_eq!(
        *order.borrow(),
        vec!["capture", "normal"],
        "target capture listener fires before normal listener"
    );
}

#[test]
fn non_bubbling_skips_ancestors() {
    let (_, parent, child) = tree();
    let order = Rc::new(RefCell::new(Vec::new()));
    let o1 = order.clone();
    add_event_listener(
        &child,
        "click",
        Rc::new(move |_| o1.borrow_mut().push("child".to_string())),
        EventListenerOptions::default(),
    );
    let o2 = order.clone();
    add_event_listener(
        &parent,
        "click",
        Rc::new(move |_| o2.borrow_mut().push("parent".to_string())),
        EventListenerOptions::default(),
    );
    let mut event = Event::new("click", false, true); // bubbles: false
    dispatch_event(&child, &mut event);
    assert_eq!(
        *order.borrow(),
        vec!["child"],
        "non-bubbling event must not invoke ancestors"
    );
}

#[test]
fn stop_propagation_halts_bubble() {
    let (_, parent, child) = tree();
    let order = Rc::new(RefCell::new(Vec::new()));
    let o1 = order.clone();
    add_event_listener(
        &child,
        "click",
        Rc::new(move |e| {
            o1.borrow_mut().push("child".to_string());
            e.stop_propagation();
        }),
        EventListenerOptions::default(),
    );
    let o2 = order.clone();
    add_event_listener(
        &parent,
        "click",
        Rc::new(move |_| o2.borrow_mut().push("parent".to_string())),
        EventListenerOptions::default(),
    );
    let mut event = Event::new("click", true, true);
    dispatch_event(&child, &mut event);
    assert_eq!(
        *order.borrow(),
        vec!["child"],
        "stopPropagation must stop ancestor bubbling"
    );
}

#[test]
fn stop_immediate_propagation_stops_remaining_and_ancestors() {
    let (_, parent, child) = tree();
    let order = Rc::new(RefCell::new(Vec::new()));
    {
        let o = order.clone();
        add_event_listener(
            &child,
            "click",
            Rc::new(move |e| {
                o.borrow_mut().push("first".to_string());
                e.stop_immediate_propagation();
            }),
            EventListenerOptions::default(),
        );
    }
    {
        let o = order.clone();
        add_event_listener(
            &child,
            "click",
            Rc::new(move |_| o.borrow_mut().push("second".to_string())),
            EventListenerOptions::default(),
        );
    }
    {
        let o = order.clone();
        add_event_listener(
            &parent,
            "click",
            Rc::new(move |_| o.borrow_mut().push("parent".to_string())),
            EventListenerOptions::default(),
        );
    }
    let mut event = Event::new("click", true, true);
    dispatch_event(&child, &mut event);
    assert_eq!(
        *order.borrow(),
        vec!["first"],
        "stopImmediatePropagation must stop same-node and ancestor listeners"
    );
}

#[test]
fn once_listener_removed_after_first_dispatch() {
    let solo = single();
    let count = Rc::new(RefCell::new(0));
    let c2 = count.clone();
    add_event_listener(
        &solo,
        "click",
        Rc::new(move |_| *c2.borrow_mut() += 1),
        EventListenerOptions {
            capture: false,
            once: true,
        },
    );
    let mut e1 = Event::new("click", true, true);
    dispatch_event(&solo, &mut e1);
    let mut e2 = Event::new("click", true, true);
    dispatch_event(&solo, &mut e2);
    assert_eq!(*count.borrow(), 1, "once listener fires exactly once");
}

#[test]
fn remove_event_listener_stops_invocation() {
    let solo = single();
    let count = Rc::new(RefCell::new(0));
    let c2 = count.clone();
    let callback: EventCallback = Rc::new(move |_| *c2.borrow_mut() += 1);
    add_event_listener(
        &solo,
        "click",
        callback.clone(),
        EventListenerOptions::default(),
    );
    remove_event_listener(&solo, "click", &callback, false);
    let mut event = Event::new("click", true, true);
    dispatch_event(&solo, &mut event);
    assert_eq!(*count.borrow(), 0, "removed listener must not fire");
}

#[test]
fn remove_matches_capture_flag() {
    // capture:true 的监听器不会被 capture:false 的 remove 删除。
    let solo = single();
    let count = Rc::new(RefCell::new(0));
    let c2 = count.clone();
    let callback: EventCallback = Rc::new(move |_| *c2.borrow_mut() += 1);
    add_event_listener(
        &solo,
        "click",
        callback.clone(),
        EventListenerOptions {
            capture: true,
            once: false,
        },
    );
    remove_event_listener(&solo, "click", &callback, false); // wrong capture flag
    let mut event = Event::new("click", true, true);
    dispatch_event(&solo, &mut event);
    assert_eq!(*count.borrow(), 1, "capture-flag mismatch must not remove");
}

#[test]
fn mutate_listeners_during_dispatch_no_panic() {
    // A 派发中移除 B；B 派发中新增 C。
    // 快照语义：A、B 都触发，C 不在本次派发的快照中。
    let (_, _, child) = tree();
    let order = Rc::new(RefCell::new(Vec::new()));
    let c_added = Rc::new(RefCell::new(false));

    // B：触发后添加 C。
    let target_b = child.clone();
    let o_b = order.clone();
    let c_added2 = c_added.clone();
    let callback_b: EventCallback = Rc::new(move |_| {
        o_b.borrow_mut().push("b".to_string());
        *c_added2.borrow_mut() = true;
        let o_c = o_b.clone();
        add_event_listener(
            &target_b,
            "click",
            Rc::new(move |_| o_c.borrow_mut().push("c".to_string())),
            EventListenerOptions::default(),
        );
    });

    // A：触发后移除 B。
    {
        let o = order.clone();
        let target_a = child.clone();
        let b_handle = callback_b.clone();
        let callback_a: EventCallback = Rc::new(move |_| {
            o.borrow_mut().push("a".to_string());
            remove_event_listener(&target_a, "click", &b_handle, false);
        });
        add_event_listener(&child, "click", callback_a, EventListenerOptions::default());
    }
    add_event_listener(&child, "click", callback_b, EventListenerOptions::default());

    let mut event = Event::new("click", true, true);
    dispatch_event(&child, &mut event);
    assert_eq!(
        *order.borrow(),
        vec!["a", "b"],
        "A removes B but B already snapshotted; C not in this dispatch"
    );
    assert!(*c_added.borrow(), "B should have added C");
}

#[test]
fn prevent_default_only_affects_cancelable() {
    let solo = single();
    let cb: EventCallback = Rc::new(|e| e.prevent_default());
    add_event_listener(&solo, "click", cb, EventListenerOptions::default());
    let mut event = Event::new("click", true, false); // cancelable: false
    dispatch_event(&solo, &mut event);
    assert!(
        !event.default_prevented(),
        "preventDefault must no-op on non-cancelable event"
    );
}

#[test]
fn dispatch_returns_false_when_canceled() {
    let solo = single();
    let cb: EventCallback = Rc::new(|e| e.prevent_default());
    add_event_listener(&solo, "click", cb, EventListenerOptions::default());
    let mut event = Event::new("click", true, true);
    assert!(
        !dispatch_event(&solo, &mut event),
        "dispatchEvent must return false when canceled"
    );
}

#[test]
fn default_prevented_visible_to_later_listeners() {
    // capture 监听器 preventDefault，at-target 监听器可见 defaultPrevented。
    let solo = single();
    let seen = Rc::new(RefCell::new(false));
    let s2 = seen.clone();
    let cb_capture: EventCallback = Rc::new(|e| e.prevent_default());
    let cb_normal: EventCallback = Rc::new(move |e| {
        if e.default_prevented() {
            *s2.borrow_mut() = true;
        }
    });
    add_event_listener(
        &solo,
        "click",
        cb_capture,
        EventListenerOptions {
            capture: true,
            once: false,
        },
    );
    add_event_listener(&solo, "click", cb_normal, EventListenerOptions::default());
    let mut event = Event::new("click", true, true);
    dispatch_event(&solo, &mut event);
    assert!(
        *seen.borrow(),
        "later listener should observe defaultPrevented"
    );
}
