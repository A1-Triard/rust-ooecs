use arena_container::Arena;
use std::any::TypeId;

struct WorldComponent {
    ty: TypeId,
    storage: Option<(usize, usize, usize, isize)>,
}

pub struct World {
    components: Vec<WorldComponent>,
    entities: Arena<usize, Vec<isize>>,
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
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Entity(usize);

impl Entity {
    pub fn new(world: &mut World) -> Self {
        world.entities.insert(|id| (Vec::new(), Entity(id)))
    }

    pub fn add_component<T: 'static>(self, world: &mut World, component: Component, t: T) {
        assert_eq!(world.components[component.0].ty, TypeId::of::<T>());
        let storage = world.components[component.0].storage.take().unwrap();
        let mut storage: Arena<isize, T> = unsafe {
            Arena::from_raw_parts(storage.0, storage.1, storage.2, storage.3)
        };
        let id = storage.insert(move |id| (t, id));
        world.components[component.0].storage = Some(storage.into_raw_parts());
        let components = &mut world.entities[self.0];
        for _ in components.len() ..= component.0 {
            components.push(-1);
        }
        components[component.0] = id;
    }
}

