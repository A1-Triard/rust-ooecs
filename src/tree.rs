use core::mem::replace;
use crate::{Entity, Component, World};

pub struct Tree {
    pub parent: Option<Entity>,
    pub last_child: Option<Entity>,
    pub next_sibling: Entity,
}

impl Tree {
    pub fn init(node: Entity, world: &mut World, tree_component: Component) {
        node.add_component::<Tree>(
            world,
            tree_component,
            Tree { parent: None, last_child: None, next_sibling: node }
        );
    }

    pub fn destroy(node: Entity, world: &mut World, tree_component: Component) {
        {
            let tree = node.component::<Tree>(world, tree_component).unwrap();
            assert!(tree.parent.is_none());
            assert_eq!(tree.next_sibling, node);
        }
        Self::destroy_raw(node, world, tree_component);
    }

    fn destroy_raw(node: Entity, world: &mut World, tree_component: Component) -> Entity {
        let tree = node.remove_component::<Tree>(world, tree_component);
        if let Some(last_child) = tree.last_child {
            let mut child = last_child;
            loop {
                let next = Self::destroy_raw(child, world, tree_component);
                child = next;
                if child == last_child { break; }
            }
        }
        tree.next_sibling
    }

    pub fn attach_first(
        parent: Entity,
        node: Entity,
        world: &mut World,
        tree_component: Component,
    ) {
        {
            let tree = node.component::<Tree>(world, tree_component).unwrap();
            assert!(tree.parent.is_none());
            assert_eq!(tree.next_sibling, node);
        }
        let parent_tree_last_child = parent.component::<Tree>(world, tree_component).unwrap().last_child;
        if let Some(last_child) = parent_tree_last_child {
            let next =
                replace(&mut last_child.component::<Tree>(world, tree_component).unwrap().next_sibling, node);
            let mut tree = node.component::<Tree>(world, tree_component).unwrap();
            tree.parent = Some(parent);
            tree.next_sibling = next;
        } else {
            node.component::<Tree>(world, tree_component).unwrap().parent = Some(parent);
            let mut parent_tree = parent.component::<Tree>(world, tree_component).unwrap();
            parent_tree.last_child = Some(node);
        }
    }

    pub fn attach_last(
        parent: Entity,
        node: Entity,
        world: &mut World,
        tree_component: Component,
    ) {
        {
            let tree = node.component::<Tree>(world, tree_component).unwrap();
            assert!(tree.parent.is_none());
            assert_eq!(tree.next_sibling, node);
        }
        let parent_tree_last_child = parent.component::<Tree>(world, tree_component).unwrap().last_child;
        if let Some(last_child) = parent_tree_last_child {
            let next =
                replace(&mut last_child.component::<Tree>(world, tree_component).unwrap().next_sibling, node);
            let mut tree = node.component::<Tree>(world, tree_component).unwrap();
            tree.parent = Some(parent);
            tree.next_sibling = next;
        } else {
            let mut tree = node.component::<Tree>(world, tree_component).unwrap();
            tree.parent = Some(parent);
        }
        let mut parent_tree = parent.component::<Tree>(world, tree_component).unwrap();
        parent_tree.last_child = Some(node);
    }

    pub fn attach_after(
        parent: Entity,
        prev: Entity,
        node: Entity,
        world: &mut World,
        tree_component: Component,
    ) {
        {
            let tree = node.component::<Tree>(world, tree_component).unwrap();
            assert!(tree.parent.is_none());
            assert_eq!(tree.next_sibling, node);
        }
        let next = replace(&mut prev.component::<Tree>(world, tree_component).unwrap().next_sibling, node);
        {
            let mut tree = node.component::<Tree>(world, tree_component).unwrap();
            tree.parent = Some(parent);
            tree.next_sibling = next;
        }
        let mut parent_tree = parent.component::<Tree>(world, tree_component).unwrap();
        if parent_tree.last_child == Some(prev) {
            parent_tree.last_child = Some(node);
        }
    }

    pub fn detach(node: Entity, world: &mut World, tree_component: Component) {
        {
            let tree = node.component::<Tree>(world, tree_component).unwrap();
            assert!(tree.parent.is_some());
        }
        let (parent, next) = {
            let mut tree = node.component::<Tree>(world, tree_component).unwrap();
            let parent = replace(&mut tree.parent, None).unwrap();
            let next = replace(&mut tree.next_sibling, node);
            (parent, next)
        };
        if node != next {
            let mut prev = node;
            loop {
                let next = prev.component::<Tree>(world, tree_component).unwrap().next_sibling;
                if next == node { break; }
                prev = next;
            }
            debug_assert_ne!(prev, node);
            prev.component::<Tree>(world, tree_component).unwrap().next_sibling = next;
            let mut parent_tree = parent.component::<Tree>(world, tree_component).unwrap();
            if parent_tree.last_child == Some(node) {
                parent_tree.last_child = Some(prev);
            }
        } else {
            let mut parent_tree = parent.component::<Tree>(world, tree_component).unwrap();
            parent_tree.last_child = None;
        }
    }
}
