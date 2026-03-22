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

#[cfg(test)]
mod tests {
    use crate::*;
    use crate::list::List;

    #[test]
    fn create_list_remove_node_and_destroy() {
        let world = &mut World::new();
        let list_component = Component::new::<List>(world);

        let node_1 = Entity::new(world);
        List::init(node_1, world, list_component);
        let node_2 = Entity::new(world);
        List::init(node_2, world, list_component);
        let node_3 = Entity::new(world);
        List::init(node_3, world, list_component);

        List::add(node_2, node_3, world, list_component);
        List::add(node_1, node_2, world, list_component);

        assert_eq!(node_1.component::<List>(world, list_component).unwrap().next, node_2);
        assert_eq!(node_2.component::<List>(world, list_component).unwrap().next, node_3);
        assert_eq!(node_3.component::<List>(world, list_component).unwrap().next, node_1);

        List::remove(node_2, node_2, world, list_component);

        assert_eq!(node_1.component::<List>(world, list_component).unwrap().next, node_3);
        assert_eq!(node_3.component::<List>(world, list_component).unwrap().next, node_1);
        assert_eq!(node_2.component::<List>(world, list_component).unwrap().next, node_2);

        List::destroy(node_3, world, list_component);

        assert!(node_1.component::<List>(world, list_component).is_none());
        assert!(node_3.component::<List>(world, list_component).is_none());

        list_component.drop_component::<List>(world);
    }
}
