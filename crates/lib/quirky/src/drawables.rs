use futures_signals::signal_vec::MutableVec;
use glam::UVec2;
use std::sync::Arc;
use crate::primitives::{DrawablePrimitive};
use crate::primitives::quad::Quads;

#[derive(Clone)]
pub enum Drawable {
    Quad(Arc<Quads>),
    Primitive(Arc<dyn DrawablePrimitive + Send + Sync>),
    ChildList(Vec<Drawable>),
    SubTree {
        transform: UVec2,
        size: UVec2,
        children: MutableVec<Drawable>,
    },
}
