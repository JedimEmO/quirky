use futures_signals::signal_vec::MutableVec;
use glam::{UVec2};


pub enum ViewNode {
    Quad { pos: UVec2, size: UVec2 },
    SubView { view: Vec<View> },
}

pub struct View {
    pub children: MutableVec<ViewNode>,
}


// for each next child rects
//  update each childs bounding box
//  call each childs render method to get its view

// Poll layout to get child rects
// with new child rects, set childrens bounding box
// poll view for widget -> impl Signal<View>
    // in poll, poll childrens views