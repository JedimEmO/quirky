use quirky_macros::widget;

#[widget]
pub struct Slab {
    #[signal]
    color: [f32; 4],
    #[signal]
    #[default("foo".to_string())]
    name: String,
}

#[cfg(test)]
mod test {
    use futures_signals::signal::always;
    use crate::widgets::slab::SlabBuilder;

    #[test]
    fn slab_builder_test() {
        let _slab = SlabBuilder::new()
            .color_signal(|| always([0.0, 0.0, 0.0, 0.0]))
            .name_signal(|| always("foo".to_string()))
            .name("bar".to_string())
            .build();
    }
}