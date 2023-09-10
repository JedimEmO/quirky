use crate::drawables::Drawable;
use crate::{LayoutBox, QuirkyAppContext, SizeConstraint};
use futures_signals::signal::{ReadOnlyMutable, Signal};
use futures_signals::signal_vec::MutableVec;

use std::sync::Arc;
use wgpu::Device;

#[async_trait::async_trait]
pub trait Widget: Send + Sync {
    fn paint(&self, device: &Device) -> Vec<Drawable>;
    fn size_constraint(&self) -> Box<dyn Signal<Item=SizeConstraint> + Unpin + Send>;
    fn set_bounding_box(&self, new_box: LayoutBox);
    fn bounding_box(&self) -> ReadOnlyMutable<LayoutBox>;
    async fn run(
        self: Arc<Self>,
        ctx: &QuirkyAppContext,
        drawable_data: MutableVec<Drawable>,
        device: &Device,
    );
}