mod builder;
mod props;
mod widget_struct;

use crate::builder::BuilderStruct;
use crate::widget_struct::WidgetStructParsed;
use convert_case::Casing;
use proc_macro::TokenStream;
use syn::parse::Parse;
use syn::spanned::Spanned;

///
/// # Example
/// ```rust,ignore
/// #[widget]
/// struct MyWidget {
///     #[signal_prop] my_property_that_can_change: i32
///     #[prop] my_static_property: f32
/// }
///
/// #[cfg(test)]
/// mod test {
/// #[test]
/// fn usage() {
///  let my_widget = MyWidgetBuilder::new().build();
/// }
/// }
/// ```
#[proc_macro_attribute]
pub fn widget(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    let struct_ = syn::parse::<syn::ItemStruct>(input).expect("failed to parse struct");
    BuilderStruct {
        widget_struct: WidgetStructParsed::from(struct_),
    }
    .into()
}
