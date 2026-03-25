use arena_container::Arena;
use std::alloc::{Layout, alloc, realloc};
use std::any::TypeId;
use std::cmp::max;
use std::ptr::{self, null_mut};

struct ComponentInfo {
    ty: TypeId,
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

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Component(isize);

impl Component {
    pub fn new<T: 'static>(base: Option<Component>, world: &mut World) -> Self {
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
}
