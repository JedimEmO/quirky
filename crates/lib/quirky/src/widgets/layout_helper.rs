use crate::widget::SizeConstraint;
use crate::LayoutBox;
use futures_signals::map_ref;
use futures_signals::signal::Signal;
use futures_signals::signal_vec::{SignalVec, SignalVecExt};

pub fn layout<TExtras: Send>(
    container_box: impl Signal<Item = LayoutBox> + Send,
    constraints: impl SignalVec<Item = Box<dyn Signal<Item = SizeConstraint> + Unpin + Send>> + Send,
    extras_signal: impl Signal<Item = TExtras> + Send,
    layout_strategy: impl Fn(&LayoutBox, &Vec<SizeConstraint>, &TExtras) -> Vec<LayoutBox> + Send,
) -> impl Signal<Item = Vec<LayoutBox>> + Send {
    let constraints = constraints.map_signal(|x| x).to_signal_cloned();

    map_ref! {
        let container_box = container_box,
        let child_constraints = constraints,
        let extras = extras_signal => {
            layout_strategy(container_box, child_constraints, extras)
        }
    }
}
