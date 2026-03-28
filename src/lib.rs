#![feature(sized_hierarchy)]

#![deny(warnings)]
#![doc(test(attr(deny(warnings))))]
#![doc(test(attr(allow(dead_code))))]
#![doc(test(attr(allow(unused_variables))))]
#![allow(clippy::collapsible_if)]

#![no_std]

extern crate alloc;

use alloc::alloc::{Layout, alloc, realloc, dealloc};
use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use components_arena::{Arena, Id, NewtypeComponentId};
use components_arena::Component as arena_Component;
use core::any::TypeId;
use core::cmp::max;
use core::marker::PointeeSized;
use core::mem::replace;
use core::ptr::{self, null_mut};
use educe::Educe;
use macro_attr_2018::macro_attr;
use phantom_type::PhantomType;

struct ComponentInfo {
    ty: TypeId,
    drop_component: Box<dyn Fn(*mut u8)>,
    archetype_unaligned_size: usize,
    archetype_size: usize,
    archetype_align: usize,
    offset: usize,
    index: usize,
    archetype_components_except_self: Vec<usize>,
    archetype_storage_ptr: *mut u8,
    archetype_storage_capacity: usize,
    archetype_storage_len: usize,
    archetype_storage_vacancy: Option<usize>,
}

macro_attr! {
    #[derive(arena_Component!)]
    struct EntityInfo {
        archetype: usize,
        index: usize,
        component_initialized: Option<Vec<bool>>,
    }
}

pub struct World<E: PointeeSized + 'static> {
    components: Vec<ComponentInfo>,
    entities: Arena<EntityInfo>,
    _phantom: PhantomType<&'static E>
}

impl<E: PointeeSized> World<E> {
    pub const fn new() -> Self {
        World {
            components: Vec::new(),
            entities: Arena::new(),
            _phantom: PhantomType::new(),
        }
    }
}

impl<E: PointeeSized> Drop for World<E> {
    fn drop(&mut self) {
        for e_info in self.entities.items().values() {
            let archetype = &self.components[e_info.archetype];
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
                let c_info = &self.components[component];
                if let Some(component_initialized) = &e_info.component_initialized {
                    if !component_initialized[c_info.index] { continue; }
                }
                let p = unsafe {
                    archetype.archetype_storage_ptr.add(archetype.archetype_size * e_info.index + c_info.offset)
                };
                (c_info.drop_component)(p);
            }
        }
        for archetype in &self.components {
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

/// An ID of a piece of [`Entity`] data and [`Entity`] archetype.
///
/// Each component defines not only component itself, but also an archetype.
/// Archetype is a collection of components. Each entity belongs to the spicific archetype.
/// To specify which components an archetype consists of, the base component notion is used.
/// Exactly, each component corresponds to archetype containing the component
/// and all base components along the chain.
#[derive(Educe)]
#[educe(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Component<E: PointeeSized + 'static>(usize, PhantomType<&'static E>);

impl<E: PointeeSized> Component<E> {
    /// Register new [`Component`] and corresponding archetype.
    ///
    /// An archetype is a collection of [`Component`]s. The registered archetype contains
    /// of corresponding component and all components of `base` archetype.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use ooecs::{World, Component}; 
    /// pub struct Position {
    ///     x: i16,
    ///     y: i16,
    /// }
    ///
    /// pub struct Velocity {
    ///     x: i16,
    ///     y: i16,
    /// }
    ///
    /// pub enum Game { }
    ///
    /// # fn main() {
    /// let mut world = <World<Game>>::new();
    /// let position = Component::new::<Position>(None, &mut world);
    /// let velocity = Component::new::<Velocity>(Some(position), &mut world);
    /// # }
    /// ```
    ///
    /// Here we define two `Component`s: `position` and `velocity`, and two corresponding archetypes.
    /// The `position` archetype contains `position` component only. The `velocity` archetype
    /// contains two components: `velocity` and `position`.
    pub fn new<T: 'static>(base: Option<Component<E>>, world: &mut World<E>) -> Self {
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
            archetype_components_except_self.push(base.0);
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
        let id = world.components.len();
        world.components.push(info);
        Component(id, PhantomType::new())
    }
}

macro_attr! {
    /// Unique identifier for an entity in a [`World`].
    ///
    /// Note that this is just an ID, not the entity itself.
    /// Further, the entity this ID refers to may no longer exist in the [`World`].
    #[derive(Educe, NewtypeComponentId!)]
    #[educe(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
    pub struct Entity<E: PointeeSized + 'static>(Id<EntityInfo>, PhantomType<&'static E>);
}

impl<E: PointeeSized> Entity<E> {
    /// Create new [`Entity`] with provided `archetype`.
    ///
    /// After creation and before using
    /// (i. e. calling [`get`](Entity::get)/[`get_mut`](Entity::get_mut) methods)
    /// all components that make up the `archetype` must be initialized using
    /// the [`add`](Entity::add) method.
    ///
    /// An [`Entity`] cannot contains [`Component`] that does not belong to the `archetype`.
    pub fn new(archetype: Component<E>, world: &mut World<E>) -> Self {
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
            archetype: archetype.0,
            index,
            component_initialized: Some(component_initialized),
        }, Entity(id, PhantomType::new())))
    }

    pub fn drop_entity(self, world: &mut World<E>) {
        let e_info = world.entities.remove(self.0);
        let archetype = &world.components[e_info.archetype];
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
            let c_info = &world.components[component];
            if let Some(component_initialized) = &e_info.component_initialized {
                if !component_initialized[c_info.index] { continue; }
            }
            let p = unsafe {
                archetype.archetype_storage_ptr.add(archetype.archetype_size * e_info.index + c_info.offset)
            };
            (c_info.drop_component)(p);
        }
        let archetype = &mut world.components[e_info.archetype];
        let cell = unsafe { archetype.archetype_storage_ptr.add(archetype.archetype_size * e_info.index) };
        unsafe { ptr::write(cell as *mut Option<usize>, archetype.archetype_storage_vacancy) };
        archetype.archetype_storage_vacancy = Some(e_info.index);
    }

    pub fn add<T: 'static>(self, component: Component<E>, world: &mut World<E>, value: T) {
        let e_info = &mut world.entities[self.0];
        let c_info = &world.components[component.0];
        assert_eq!(c_info.ty, TypeId::of::<T>(), "component type mismatch");
        assert!(!replace(&mut e_info.component_initialized.as_mut().unwrap()[c_info.index], true));
        if e_info.component_initialized.as_ref().unwrap().iter().all(|&x| x) {
            e_info.component_initialized = None;
        }
        let c_index = c_info.index;
        let c_offset = c_info.offset;
        let archetype = &world.components[e_info.archetype];
        assert!(
               component.0 == e_info.archetype
            || archetype.archetype_components_except_self.get(c_index).copied() == Some(component.0)
        );
        let p = unsafe {
            archetype.archetype_storage_ptr.add(archetype.archetype_size * e_info.index + c_offset) as *mut T
        };
        unsafe { ptr::write(p, value); }
    }

    pub fn get<T: 'static>(self, component: Component<E>, world: &World<E>) -> Option<&T> {
        let e_info = &world.entities[self.0];
        let c_info = &world.components[component.0];
        assert_eq!(c_info.ty, TypeId::of::<T>(), "component type mismatch");
        assert!(e_info.component_initialized.is_none());
        let archetype = &world.components[e_info.archetype];
        if
               component.0 != e_info.archetype
            && archetype.archetype_components_except_self.get(c_info.index).copied() != Some(component.0)
        {
            return None;
        }
        let p = unsafe {
            archetype.archetype_storage_ptr.add(archetype.archetype_size * e_info.index + c_info.offset)
        };
        Some(unsafe { &*(p as *mut T) })
    }

    pub fn get_mut<T: 'static>(self, component: Component<E>, world: &mut World<E>) -> Option<&mut T> {
        let e_info = &world.entities[self.0];
        let c_info = &world.components[component.0];
        assert_eq!(c_info.ty, TypeId::of::<T>(), "component type mismatch");
        assert!(e_info.component_initialized.is_none());
        let archetype = &world.components[e_info.archetype];
        assert!(
               component.0 == e_info.archetype
            || archetype.archetype_components_except_self.get(c_info.index).copied() == Some(component.0)
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
    use core::sync::atomic::{AtomicIsize, Ordering};

    enum X { }

    struct Position {
        x: i16,
    }

    struct Velocity {
        x: i16,
    }

    #[test]
    fn create_world_reg_component_drop_world() {
        let mut world = <World<X>>::new();
        let _position = Component::new::<Position>(None, &mut world);
        drop(world);
    }

    #[test]
    fn create_entity_modify_check() {
        let world = &mut <World<X>>::new();
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
        let mut world = <World<X>>::new();
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
