use crate::primitives::Quads;
use futures_signals::signal_vec::MutableVec;
use glam::UVec2;
use std::sync::Arc;

#[derive(Clone)]
pub enum Drawable {
    Quad(Arc<Quads>),
    ChildList(Vec<Drawable>),
    SubTree {
        transform: UVec2,
        size: UVec2,
        children: MutableVec<Drawable>,
    },
}
