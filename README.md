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
```

Next, create `World` and register components:

```rust
let world = &mut World::new();
let position = Component::new::<Position>(world);
let velocity = Component::new::<Velocity>(world);
let mobs = Component::new::<List>(world);
```

The `mobs` component is needed for organizing entities in linear list.

Lets create some test entities and attach components to them:

```rust
let player = Entity::new(world);
let mob = Entity::new(world);

player.add_component(world, position, Position { x: 0, y: 0 });
player.add_component(world, velocity, Velocity { x: 1, y: 0 });
List::init(player, world, mobs);

mob.add_component(world, position, Position { x: 10, y: 0 });
mob.add_component(world, velocity, Velocity { x: -1, y: 0 });
List::init(mob, world, mobs);
```

Put entities in common mobs list:

```rust
List::add(player, mob, world, mobs);
```

Create `Movement` system using `basic-oop` library:

```rust
import! { pub movement:
    use [obj basic_oop::obj];
    use ooecs::{Entity, Component, World};
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
            let (vx, vy) = {
                let velocity = entity.component::<Velocity>(world, movement.velocity).unwrap();
                (velocity.x, velocity.y)
            };
            {
                let mut position = entity.component::<Position>(world, movement.position).unwrap();
                position.x += vx;
                position.y += vy;
            }
            entity = entity.component::<List>(world, movement.mobs).unwrap().next;
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

Finally, do some cleanup:
```rust
position.drop_component::<Position>(world);
velocity.drop_component::<Velocity>(world);
mobs.drop_component::<List>(world);
```

Full code:

```rust
#![feature(macro_metavar_expr_concat)]

mod game {
    use basic_oop::{Vtable, import, class_unsafe};
    use ooecs::list::List;
    use std::rc::Rc;

    pub struct Position {
        pub x: i16,
        pub y: i16,
    }

    pub struct Velocity {
        pub x: i16,
        pub y: i16,
    }

    import! { pub movement:
        use [obj basic_oop::obj];
        use ooecs::{Entity, Component, World};
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
                let (vx, vy) = {
                    let velocity = entity.component::<Velocity>(world, movement.velocity).unwrap();
                    (velocity.x, velocity.y)
                };
                {
                    let mut position = entity.component::<Position>(world, movement.position).unwrap();
                    position.x += vx;
                    position.y += vy;
                }
                entity = entity.component::<List>(world, movement.mobs).unwrap().next;
                if entity == start { break; }
            }
        }
    }
}

use game::*;
use ooecs::{Entity, Component, World};
use ooecs::list::List;

fn main() {
    let world = &mut World::new();
    let position = Component::new::<Position>(world);
    let velocity = Component::new::<Velocity>(world);
    let mobs = Component::new::<List>(world);

    let player = Entity::new(world);
    let mob = Entity::new(world);

    player.add_component(world, position, Position { x: 0, y: 0 });
    player.add_component(world, velocity, Velocity { x: 1, y: 0 });
    List::init(player, world, mobs);

    mob.add_component(world, position, Position { x: 10, y: 0 });
    mob.add_component(world, velocity, Velocity { x: -1, y: 0 });
    List::init(mob, world, mobs);
    List::add(player, mob, world, mobs);

    let movement = Movement::new(mobs, position, velocity);
    movement.run(player, world);

    position.drop_component::<Position>(world);
    velocity.drop_component::<Velocity>(world);
    mobs.drop_component::<List>(world);
}
```
