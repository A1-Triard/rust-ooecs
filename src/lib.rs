use arena_container::Arena;
use panicking::panicking;
use std::any::TypeId;
use std::mem::replace;
use std::ops::{Deref, DerefMut};

struct WorldComponent {
    ty: TypeId,
    storage: Option<(usize, usize, usize, isize)>,
}

pub struct World {
    components: Vec<WorldComponent>,
    entities: Arena<usize, Vec<isize>>,
}

impl Drop for World {
    fn drop(&mut self) {
        if !panicking() {
            assert!(self.components.iter().all(|x| x.storage.is_none()));
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Component(usize);

impl World {
    pub fn new() -> Self {
        World {
            components: Vec::new(),
            entities: Arena::new(),
        }
    }
}

impl Component {
    pub fn new<T: 'static>(world: &mut World) -> Self {
        let storage: Arena<isize, T> = Arena::new();
        let storage = storage.into_raw_parts();
        let ty = TypeId::of::<T>();
        let id = world.components.len();
        world.components.push(WorldComponent {
            ty,
            storage: Some(storage),
        });
        Component(id)
    }

    pub fn drop_component<T: 'static>(self, world: &mut World) {
        assert_eq!(world.components[self.0].ty, TypeId::of::<T>());
        let storage = world.components[self.0].storage.take().unwrap();
        let storage: Arena<isize, T> = unsafe {
            Arena::from_raw_parts(storage.0, storage.1, storage.2, storage.3)
        };
        drop(storage);
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Entity(usize);

pub struct ComponentRef<'a, T: 'static> {
    world: &'a mut World,
    component: Component,
    storage: Option<Arena<isize, T>>,
    id: isize,
}

impl<'a, T: 'static> Drop for ComponentRef<'a, T> {
    fn drop(&mut self) {
        let storage = self.storage.take().unwrap().into_raw_parts();
        self.world.components[self.component.0].storage = Some(storage);
    }
}

impl<'a, T: 'static> Deref for ComponentRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.storage.as_ref().unwrap()[self.id]
    }
}

impl<'a, T: 'static> DerefMut for ComponentRef<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.storage.as_mut().unwrap()[self.id]
    }
}

impl Entity {
    pub fn new(world: &mut World) -> Self {
        world.entities.insert(|id| (Vec::new(), Entity(id)))
    }

    pub fn drop_entity(self, world: &mut World) {
        let components = world.entities.remove(self.0);
        assert!(components.iter().all(|&x| x < 0));
    }

    pub fn add_component<T: 'static>(self, world: &mut World, component: Component, t: T) {
        assert_eq!(world.components[component.0].ty, TypeId::of::<T>());
        let components = &mut world.entities[self.0];
        for _ in components.len() ..= component.0 {
            components.push(-1);
        }
        assert!(components[component.0] < 0);
        let storage = world.components[component.0].storage.take().unwrap();
        let mut storage: Arena<isize, T> = unsafe {
            Arena::from_raw_parts(storage.0, storage.1, storage.2, storage.3)
        };
        let id = storage.insert(move |id| (t, id));
        world.components[component.0].storage = Some(storage.into_raw_parts());
        components[component.0] = id;
    }

    pub fn remove_component<T: 'static>(self, world: &mut World, component: Component) -> T {
        assert_eq!(world.components[component.0].ty, TypeId::of::<T>());
        let components = &mut world.entities[self.0];
        for _ in components.len() ..= component.0 {
            components.push(-1);
        }
        let id = replace(&mut components[component.0], -1);
        assert!(id >= 0);
        let storage = world.components[component.0].storage.take().unwrap();
        let mut storage: Arena<isize, T> = unsafe {
            Arena::from_raw_parts(storage.0, storage.1, storage.2, storage.3)
        };
        let res = storage.remove(id);
        world.components[component.0].storage = Some(storage.into_raw_parts());
        res
    }

    pub fn component<T: 'static>(self, world: &mut World, component: Component) -> Option<ComponentRef<'_, T>> {
        assert_eq!(world.components[component.0].ty, TypeId::of::<T>());
        let storage = world.components[component.0].storage.take().unwrap();
        let storage: Arena<isize, T> = unsafe {
            Arena::from_raw_parts(storage.0, storage.1, storage.2, storage.3)
        };
        let id = world.entities[self.0].get(component.0).copied();
        match id {
            Some(id) if id >= 0 => Some(ComponentRef { world, component, storage: Some(storage), id }),
            _ => {
                world.components[component.0].storage = Some(storage.into_raw_parts());
                None
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    struct TestComponent {
        value: i8,
    }

    #[test]
    fn add_modify_remove_component() {
        let world = &mut World::new();
        let component = Component::new::<TestComponent>(world);
        let entity = Entity::new(world);
        assert!(entity.component::<TestComponent>(world, component).is_none());
        entity.add_component::<TestComponent>(world, component, TestComponent { value: 7 });
        assert_eq!(entity.component::<TestComponent>(world, component).unwrap().value, 7);
        entity.component::<TestComponent>(world, component).unwrap().value = 8;
        assert_eq!(entity.component::<TestComponent>(world, component).unwrap().value, 8);
        assert_eq!(entity.remove_component::<TestComponent>(world, component).value, 8);
        entity.drop_entity(world);
        component.drop_component::<TestComponent>(world);
    }
}
