use std::sync::{Arc, Weak};

/*
component AbstractGoobComponentBuilder {
    gwah: s67;

    event enter_tree() {
        Piru::log("Hello, world!");
    }

    event render_update() {};

    event physics_update() {};

    event exit_tree() {};
}


component RigidBody {
    collider: Mesh;
    velocity: Vec3<f32>;
    position: Vec3<f32>;
}
 */
pub struct PithValue {}

pub struct PithFunction {
    //
}

impl PithFunction {
    fn invoke(&self, args: Vec<PithValue>) -> Result<PithValue, ()> {
        Err(())
    }
}

pub struct Component {
    awake: Option<PithFunction>,

    fields: Vec<PithValue>,
    events: Vec<PithFunction>,
}

pub struct Node {
    children: Vec<Arc<Node>>,
    parent: Option<Weak<Node>>,
    name: String,
    components: Vec<Component>,
}
