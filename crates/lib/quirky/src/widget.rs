use crate::drawables::Drawable;
use crate::{LayoutBox, SizeConstraint};
use futures_signals::signal::{ReadOnlyMutable, Signal};
use futures_signals::signal_vec::MutableVec;

use crate::quirky_app_context::QuirkyAppContext;
use glam::UVec2;
use std::sync::Arc;
use uuid::Uuid;
use wgpu::Device;

#[async_trait::async_trait]
pub trait Widget: Send + Sync {
    fn id(&self) -> Uuid;
    fn paint(&self, device: &Device) -> Vec<Drawable>;
    fn size_constraint(&self) -> Box<dyn Signal<Item = SizeConstraint> + Unpin + Send>;
    fn set_bounding_box(&self, new_box: LayoutBox);
    fn bounding_box(&self) -> ReadOnlyMutable<LayoutBox>;
    fn get_widget_at(&self, pos: UVec2, path: Vec<Uuid>) -> Option<Vec<Uuid>>;

    async fn run(
        self: Arc<Self>,
        ctx: &QuirkyAppContext,
        drawable_data: MutableVec<Drawable>,
        device: &Device,
    );
}
