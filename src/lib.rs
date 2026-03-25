#![allow(clippy::collapsible_if)]

use arena_container::Arena;
use std::alloc::{Layout, alloc, realloc, dealloc};
use std::any::TypeId;
use std::cmp::max;
use std::mem::replace;
use std::ptr::{self, null_mut};

struct ComponentInfo {
    ty: TypeId,
    drop_component: Box<dyn Fn(*mut u8)>,
    archetype_unaligned_size: usize,
    archetype_size: usize,
    archetype_align: usize,
    offset: usize,
    index: usize,
    archetype_components_except_self: Vec<Component>,
    archetype_storage_ptr: *mut u8,
    archetype_storage_capacity: usize,
    archetype_storage_len: usize,
    archetype_storage_vacancy: Option<usize>,
}

struct EntityInfo {
    archetype: Component,
    index: usize,
    component_initialized: Option<Vec<bool>>,
}

pub struct World {
    components: Arena<isize, ComponentInfo>,
    entities: Arena<isize, EntityInfo>,
}

impl World {
    pub const fn new() -> Self {
        World {
            components: Arena::new(),
            entities: Arena::new()
        }
    }
}

impl Drop for World {
    fn drop(&mut self) {
        for e_info in self.entities.items().values() {
            let archetype = &self.components[e_info.archetype.0];
            let component_initialized = if let Some(component_initialized) = &e_info.component_initialized {
                component_initialized[archetype.index]
            } else {
                true
            };
            if component_initialized {
                let p = unsafe {
                    archetype.archetype_storage_ptr.add(
                        archetype.archetype_size * e_info.index + archetype.offset
                    )
                };
                (archetype.drop_component)(p);
            }
            for &component in &archetype.archetype_components_except_self {
                let c_info = &self.components[component.0];
                if let Some(component_initialized) = &e_info.component_initialized {
                    if !component_initialized[c_info.index] { continue; }
                }
                let p = unsafe {
                    archetype.archetype_storage_ptr.add(archetype.archetype_size * e_info.index + c_info.offset)
                };
                (c_info.drop_component)(p);
            }
        }
        for archetype in self.components.items().values() {
            if !archetype.archetype_storage_ptr.is_null() {
                let size = archetype.archetype_size.checked_mul(archetype.archetype_storage_capacity).unwrap();
                unsafe { dealloc(
                    archetype.archetype_storage_ptr, 
                    Layout::from_size_align(size, archetype.archetype_align).unwrap(),
                ); }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Component(isize);

impl Component {
    pub fn new<T: 'static>(base: Option<Component>, world: &mut World) -> Self {
        let drop_component = Box::new(|p: *mut u8| {
            drop(unsafe { ptr::read(p as *mut T) });
        });
        let info = if let Some(base) = base {
            let base_info = &world.components[base.0];
            let archetype_align = max(
                max(base_info.archetype_align, align_of::<T>()),
                align_of::<Option<usize>>()
            );
            let size = base_info.archetype_unaligned_size;
            let size = (size.checked_add(align_of::<T>() - 1).unwrap() / align_of::<T>()) * align_of::<T>();
            let offset = size;
            let size = size.checked_add(size_of::<T>()).unwrap();
            let archetype_unaligned_size = size;
            let size = max(size, size_of::<Option<usize>>());
            let size = (size.checked_add(archetype_align - 1).unwrap() / archetype_align) * archetype_align;
            let archetype_size = size;
            let index = base_info.index.checked_add(1).unwrap();
            let mut archetype_components_except_self = base_info.archetype_components_except_self.clone();
            archetype_components_except_self.reserve_exact(1);
            archetype_components_except_self.push(base);
            ComponentInfo {
                ty: TypeId::of::<T>(),
                drop_component,
                archetype_unaligned_size,
                archetype_size,
                archetype_align,
                offset,
                index,
                archetype_components_except_self,
                archetype_storage_ptr: null_mut(),
                archetype_storage_capacity: 0,
                archetype_storage_len: 0,
                archetype_storage_vacancy: None,
            }
        } else {
            ComponentInfo {
                ty: TypeId::of::<T>(),
                drop_component,
                archetype_unaligned_size: max(size_of::<T>(), align_of::<T>()),
                archetype_size: max(size_of::<T>(), align_of::<T>()),
                archetype_align: align_of::<T>(),
                offset: 0,
                index: 0,
                archetype_components_except_self: Vec::new(),
                archetype_storage_ptr: null_mut(),
                archetype_storage_capacity: 0,
                archetype_storage_len: 0,
                archetype_storage_vacancy: None,
            }
        };
        world.components.insert(|id| (info, Component(id)))
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Entity(isize);

impl Entity {
    pub fn new(archetype: Component, world: &mut World) -> Self {
        let info = &mut world.components[archetype.0];
        let index = if let Some(vacancy) = info.archetype_storage_vacancy {
            let cell = unsafe { info.archetype_storage_ptr.add(info.archetype_size * vacancy) };
            let new_vacancy = unsafe { ptr::read(cell as *mut Option<usize>) };
            info.archetype_storage_vacancy = new_vacancy;
            vacancy
        } else {
            if info.archetype_storage_len == info.archetype_storage_capacity {
                let new_capacity = if info.archetype_storage_capacity == 0 {
                    1
                } else {
                    info.archetype_storage_capacity.saturating_mul(2)
                };
                assert!(new_capacity > info.archetype_storage_capacity);
                let new_ptr = if info.archetype_storage_ptr.is_null() {
                    let new_size = info.archetype_size.checked_mul(new_capacity).unwrap();
                    unsafe { alloc(Layout::from_size_align(new_size, info.archetype_align).unwrap()) }
                } else {
                    let old_size = info.archetype_size.checked_mul(info.archetype_storage_capacity).unwrap();
                    let new_size = info.archetype_size.checked_mul(new_capacity).unwrap();
                    unsafe { realloc(
                        info.archetype_storage_ptr, 
                        Layout::from_size_align(old_size, info.archetype_align).unwrap(),
                        new_size
                    ) }
                };
                assert!(!new_ptr.is_null());
                info.archetype_storage_capacity = new_capacity;
                info.archetype_storage_ptr = new_ptr;
            }
            let index = info.archetype_storage_len;
            info.archetype_storage_len += 1;
            index
        };
        let component_initialized = vec![
            false;
            info.archetype_components_except_self.len().checked_add(1).unwrap()
        ];
        world.entities.insert(|id| (EntityInfo {
            archetype,
            index,
            component_initialized: Some(component_initialized),
        }, Entity(id)))
    }

    pub fn drop_entity(self, world: &mut World) {
        let e_info = world.entities.remove(self.0);
        let archetype = &world.components[e_info.archetype.0];
        let component_initialized = if let Some(component_initialized) = &e_info.component_initialized {
            component_initialized[archetype.index]
        } else {
            true
        };
        if component_initialized {
            let p = unsafe {
                archetype.archetype_storage_ptr.add(
                    archetype.archetype_size * e_info.index + archetype.offset
                )
            };
            (archetype.drop_component)(p);
        }
        for &component in &archetype.archetype_components_except_self {
            let c_info = &world.components[component.0];
            if let Some(component_initialized) = &e_info.component_initialized {
                if !component_initialized[c_info.index] { continue; }
            }
            let p = unsafe {
                archetype.archetype_storage_ptr.add(archetype.archetype_size * e_info.index + c_info.offset)
            };
            (c_info.drop_component)(p);
        }
        let archetype = &mut world.components[e_info.archetype.0];
        let cell = unsafe { archetype.archetype_storage_ptr.add(archetype.archetype_size * e_info.index) };
        unsafe { ptr::write(cell as *mut Option<usize>, archetype.archetype_storage_vacancy) };
        archetype.archetype_storage_vacancy = Some(e_info.index);
    }

    pub fn add<T: 'static>(self, component: Component, world: &mut World, value: T) {
        let e_info = &mut world.entities[self.0];
        let c_info = &world.components[component.0];
        assert_eq!(c_info.ty, TypeId::of::<T>(), "component type mismatch");
        assert!(!replace(&mut e_info.component_initialized.as_mut().unwrap()[c_info.index], true));
        if e_info.component_initialized.as_ref().unwrap().iter().all(|&x| x) {
            e_info.component_initialized = None;
        }
        let c_index = c_info.index;
        let c_offset = c_info.offset;
        let archetype = &world.components[e_info.archetype.0];
        assert!(
               component == e_info.archetype
            || archetype.archetype_components_except_self.get(c_index).copied() == Some(component)
        );
        let p = unsafe {
            archetype.archetype_storage_ptr.add(archetype.archetype_size * e_info.index + c_offset) as *mut T
        };
        unsafe { ptr::write(p, value); }
    }

    pub fn get<T: 'static>(self, component: Component, world: &World) -> Option<&T> {
        let e_info = &world.entities[self.0];
        let c_info = &world.components[component.0];
        assert_eq!(c_info.ty, TypeId::of::<T>(), "component type mismatch");
        assert!(e_info.component_initialized.is_none());
        let archetype = &world.components[e_info.archetype.0];
        if
               component != e_info.archetype
            && archetype.archetype_components_except_self.get(c_info.index).copied() != Some(component)
        {
            return None;
        }
        let p = unsafe {
            archetype.archetype_storage_ptr.add(archetype.archetype_size * e_info.index + c_info.offset)
        };
        Some(unsafe { &*(p as *mut T) })
    }

    pub fn get_mut<T: 'static>(self, component: Component, world: &mut World) -> Option<&mut T> {
        let e_info = &world.entities[self.0];
        let c_info = &world.components[component.0];
        assert_eq!(c_info.ty, TypeId::of::<T>(), "component type mismatch");
        assert!(e_info.component_initialized.is_none());
        let archetype = &world.components[e_info.archetype.0];
        assert!(
               component == e_info.archetype
            || archetype.archetype_components_except_self.get(c_info.index).copied() == Some(component)
        );
        let p = unsafe {
            archetype.archetype_storage_ptr.add(archetype.archetype_size * e_info.index + c_info.offset)
        };
        Some(unsafe { &mut *(p as *mut T) })
    }
}

#[cfg(test)]
mod tests {
    use crate::{World, Entity, Component};
    use std::sync::atomic::{AtomicIsize, Ordering};

    struct Position {
        x: i16,
    }

    struct Velocity {
        x: i16,
    }

    #[test]
    fn create_world_reg_component_drop_world() {
        let mut world = World::new();
        let _position = Component::new::<Position>(None, &mut world);
        drop(world);
    }

    #[test]
    fn create_entity_modify_check() {
        let world = &mut World::new();
        let position = Component::new::<Position>(None, world);
        let velocity = Component::new::<Velocity>(Some(position), world);
        let entity = Entity::new(velocity, world);
        entity.add(position, world, Position { x: 0 });
        entity.add(velocity, world, Velocity { x: 1 });
        assert_eq!(entity.get::<Position>(position, world).unwrap().x, 0);
        entity.get_mut::<Position>(position, world).unwrap().x = 10;
        assert_eq!(entity.get::<Position>(position, world).unwrap().x, 10);
        assert_eq!(entity.get::<Velocity>(velocity, world).unwrap().x, 1);
        entity.get_mut::<Velocity>(velocity, world).unwrap().x = -1;
        assert_eq!(entity.get::<Velocity>(velocity, world).unwrap().x, -1);
    }

    static COMPONENT_IMPL_DROP_ALIVE: AtomicIsize = AtomicIsize::new(0);

    struct ComponentImplDrop;

    impl ComponentImplDrop {
        pub fn new() -> Self {
            COMPONENT_IMPL_DROP_ALIVE.fetch_add(1, Ordering::Relaxed);
            ComponentImplDrop
        }
    }

    impl Drop for ComponentImplDrop {
        fn drop(&mut self) {
            COMPONENT_IMPL_DROP_ALIVE.fetch_sub(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn drop_components() {
        let mut world = World::new();
        let component = Component::new::<ComponentImplDrop>(None, &mut world);
        let entity_1 = Entity::new(component, &mut world);
        entity_1.add(component, &mut world, ComponentImplDrop::new());
        let entity_2 = Entity::new(component, &mut world);
        entity_2.add(component, &mut world, ComponentImplDrop::new());
        assert_eq!(COMPONENT_IMPL_DROP_ALIVE.load(Ordering::Relaxed), 2);
        entity_1.drop_entity(&mut world);
        assert_eq!(COMPONENT_IMPL_DROP_ALIVE.load(Ordering::Relaxed), 1);
        drop(world);
        assert_eq!(COMPONENT_IMPL_DROP_ALIVE.load(Ordering::Relaxed), 0);
    }
}
