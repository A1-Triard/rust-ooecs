use core::mem::replace;
use crate::{Entity, Component, World};

pub struct List {
    pub next: Entity,
}

impl List {
    pub fn init(node: Entity, world: &mut World, list_component: Component) {
        node.add_component::<List>(world, list_component, List { next: node });
    }

    pub fn destroy(node: Entity, world: &mut World, list_component: Component) {
        let mut n = node;
        loop {
            let list = n.remove_component::<List>(world, list_component);
            n = list.next;
            if n == node { break; }
        }
    }

    pub fn add(prev: Entity, node: Entity, world: &mut World, list_component: Component) {
        let mut last = node;
        loop {
            let next = last.component::<List>(world, list_component).unwrap().next;
            if next == node { break; }
            last = next;
        }
        let next = replace(&mut prev.component::<List>(world, list_component).unwrap().next, node);
        last.component::<List>(world, list_component).unwrap().next = next;
    }

    pub fn remove(from: Entity, to: Entity, world: &mut World, list_component: Component) {
        let mut prev = to;
        loop {
            let next = prev.component::<List>(world, list_component).unwrap().next;
            if next == from { break; }
            prev = next;
        }
        assert_ne!(prev, to);
        let next = replace(&mut to.component::<List>(world, list_component).unwrap().next, from);
        prev.component::<List>(world, list_component).unwrap().next = next;
    }
}
