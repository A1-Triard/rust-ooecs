![maintenance: actively developed](https://img.shields.io/badge/maintenance-actively--developed-brightgreen.svg)

# ooecs

Highly non-traditional entity-component-system implementation where logic is encapsulated in object-oriented
systems.

While components remain lean and data-focused,
systems are treated as first-class objects that can maintain their own internal state across ticks.

It is designed for cases, where simple "linear" systems are not good enough.

## Example: mimicry of traditional ECS

First, define components data:

```rust
pub struct Position {
    pub x: i16,
    pub y: i16,
}

pub struct Velocity {
    pub x: i16,
    pub y: i16,
}

pub struct List {
    pub next: Entity,
}
```

The `List` component is needed for organizing entities in linear list.

Next, create `World` and register components:

```rust
let world = &mut World::new();
let position = Component::new::<Position>(None, world);
let velocity = Component::new::<Velocity>(Some(position), world);
let mobs = Component::new::<List>(Some(velocity), world);
```

Each component defines not only component itself, but also an archetype.
Archetype is a collection of components. Each entity belongs to the spicific archetype.
To specify which components an archetype consists of, the base component notion is used.
Exactly, each component corresponds to archetype containing the component and all base
components along the chain.

In the example `position` component does not have base component, and defines an archetype
consisting of `position` component only. The `velocity` component has `position` as its base.
So, corresponding archetype consists of `velocity` and `position` components. At last,
`mobs` component has `velocity` as its base, so, `mobs` archetype consists of three components:
`mobs`, `velocity`, and `position`.


Lets create some test entities and attach components to them:

```rust
let player = Entity::new(mobs, world);
let mob = Entity::new(mobs, world);

player.add(position, world, Position { x: 0, y: 0 });
player.add(velocity, world, Velocity { x: 1, y: 0 });
player.add(mobs, world, List { next: mob });

mob.add(position, world, Position { x: 10, y: 0 });
mob.add(velocity, world, Velocity { x: -1, y: 0 });
mob.add(mobs, world, List { next: player });
```

Here we create two entities both with `mobs` archetype and attach all required components to them.

Create `Movement` system using `basic-oop` library:

```rust
import! { pub movement:
    use [obj basic_oop::obj];
    use ooecs::{Entity, World};
}

#[class_unsafe(inherits_Obj)]
pub struct Movement {
    mobs: Component,
    position: Component,
    velocity: Component,
    #[non_virt]
    run: fn(start: Entity, world: &mut World),
}

impl Movement {
    pub fn new(
        mobs: Component,
        position: Component,
        velocity: Component,
    ) -> Rc<dyn IsMovement> {
        Rc::new(unsafe { Self::new_raw(
            mobs,
            position,
            velocity,
            MOVEMENT_VTABLE.as_ptr()
        ) })
    }

    pub unsafe fn new_raw(
        mobs: Component,
        position: Component,
        velocity: Component,
        vtable: Vtable,
    ) -> Self {
        Movement {
            obj: unsafe { Obj::new_raw(vtable) },
            mobs,
            position,
            velocity,
        }
    }

    pub fn run_impl(this: &Rc<dyn IsMovement>, start: Entity, world: &mut World) {
        let movement = this.movement();
        let mut entity = start;
        loop {
            let velocity = entity.get::<Velocity>(movement.velocity, world).unwrap();
            let (vx, vy) = (velocity.x, velocity.y);
            let position = entity.get_mut::<Position>(movement.position, world).unwrap();
            position.x += vx;
            position.y += vy;
            entity = entity.get::<List>(movement.mobs, world).unwrap().next;
            if entity == start { break; }
        }
    }
}
```

Now we can run the system:

```rust
let movement = Movement::new(mobs, position, velocity);
movement.run(player, world);
```

Full code:

```rust
#![feature(macro_metavar_expr_concat)]

mod game {
    use basic_oop::{Vtable, import, class_unsafe};
    use std::rc::Rc;
    use ooecs::{Entity, Component};

    pub struct Position {
        pub x: i16,
        pub y: i16,
    }

    pub struct Velocity {
        pub x: i16,
        pub y: i16,
    }

    pub struct List {
        pub next: Entity,
    }

    import! { pub movement:
        use [obj basic_oop::obj];
        use ooecs::{Entity, World};
    }

    #[class_unsafe(inherits_Obj)]
    pub struct Movement {
        mobs: Component,
        position: Component,
        velocity: Component,
        #[non_virt]
        run: fn(start: Entity, world: &mut World),
    }

    impl Movement {
        pub fn new(
            mobs: Component,
            position: Component,
            velocity: Component,
        ) -> Rc<dyn IsMovement> {
            Rc::new(unsafe { Self::new_raw(
                mobs,
                position,
                velocity,
                MOVEMENT_VTABLE.as_ptr()
            ) })
        }

        pub unsafe fn new_raw(
            mobs: Component,
            position: Component,
            velocity: Component,
            vtable: Vtable,
        ) -> Self {
            Movement {
                obj: unsafe { Obj::new_raw(vtable) },
                mobs,
                position,
                velocity,
            }
        }

        pub fn run_impl(this: &Rc<dyn IsMovement>, start: Entity, world: &mut World) {
            let movement = this.movement();
            let mut entity = start;
            loop {
                let velocity = entity.get::<Velocity>(movement.velocity, world).unwrap();
                let (vx, vy) = (velocity.x, velocity.y);
                let position = entity.get_mut::<Position>(movement.position, world).unwrap();
                position.x += vx;
                position.y += vy;
                entity = entity.get::<List>(movement.mobs, world).unwrap().next;
                if entity == start { break; }
            }
        }
    }
}

use game::*;
use ooecs::{Entity, Component, World};

fn main() {
    let world = &mut World::new();
    let position = Component::new::<Position>(None, world);
    let velocity = Component::new::<Velocity>(Some(position), world);
    let mobs = Component::new::<List>(Some(velocity), world);

    let player = Entity::new(mobs, world);
    let mob = Entity::new(mobs, world);

    player.add(position, world, Position { x: 0, y: 0 });
    player.add(velocity, world, Velocity { x: 1, y: 0 });
    player.add(mobs, world, List { next: mob });

    mob.add(position, world, Position { x: 10, y: 0 });
    mob.add(velocity, world, Velocity { x: -1, y: 0 });
    mob.add(mobs, world, List { next: player });

    let movement = Movement::new(mobs, position, velocity);
    movement.run(player, world);
}
```
