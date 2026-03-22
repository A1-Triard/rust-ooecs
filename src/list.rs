use crate::{Entity, Component, World};

pub struct List {
    pub next: Entity,
}

impl List {
    pub fn insert(head: &mut Option<Entity>, node: Entity, world: &mut World, list_component: Component) {
        if let &mut Some(head_entity) = head {
            let mut prev = head_entity;
            loop {
                let next = prev.component::<List>(world, list_component).unwrap().next;
                if next == head_entity { break; }
                prev = next;
            }
            prev.component::<List>(world, list_component).unwrap().next = node;
            node.add_component::<List>(world, list_component, List { next: head_entity });
            *head = Some(node);
        } else {
            node.add_component::<List>(world, list_component, List { next: node });
            *head = Some(node);
        }
    }

    pub fn remove(head: &mut Option<Entity>, node: Entity, world: &mut World, list_component: Component) {
        let mut prev = node;
        loop {
            let next = prev.component::<List>(world, list_component).unwrap().next;
            if next == node { break; }
            prev = next;
        }
        if prev == node {
            node.remove_component::<List>(world, list_component);
            *head = None;
        } else {
            let next = node.remove_component::<List>(world, list_component).next;
            prev.component::<List>(world, list_component).unwrap().next = next;
            if *head == Some(node) {
                *head = Some(next);
            }
        }
    }
}
