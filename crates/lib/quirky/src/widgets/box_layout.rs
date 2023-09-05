use std::sync::Arc;
use futures_signals::signal::Mutable;
use futures_signals::signal_vec::{Always, MutableVec, SignalVec};
use crate::widget::Widget;


#[cfg(test)]
mod test {
    use std::sync::Arc;
    use futures_signals::signal_vec::MutableVec;
    use quirky_macros::widget;
    use crate::widget::Widget;
    // use crate::widgets::box_layout::{BoxLayout, BoxLayoutBuilder};

    #[widget]
    struct BoxLayout {}

    #[test]
    fn box_layout_usage() {
        let box_layout = BoxLayoutProps::new();

        // let box_layout = BoxLayout::new()
        //     .children_signal_vec(children.signal_vec_cloned())
        //     .build();
    }
}