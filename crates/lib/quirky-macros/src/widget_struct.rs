use crate::props::{CallbackProp, FnSignalProp, Prop};
use proc_macro2::Ident;
use syn::{Field, ItemStruct};

#[derive(Clone)]
pub(crate) struct WidgetStructParsed {
    pub ident: Ident,
    pub signal_props: Vec<FnSignalProp>,
    pub props: Vec<Prop>,
    pub plain_fields: Vec<Field>,
    pub callbacks: Vec<CallbackProp>,
}

impl From<ItemStruct> for WidgetStructParsed {
    fn from(struct_: ItemStruct) -> Self {
        let signal_props = struct_
            .fields
            .iter()
            .filter(|f| {
                f.attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("signal_prop"))
            })
            .cloned()
            .collect::<Vec<_>>();

        let props = struct_
            .fields
            .iter()
            .filter(|f| f.attrs.iter().any(|attr| attr.path().is_ident("prop")))
            .cloned()
            .collect::<Vec<_>>();

        let callbacks = struct_
            .fields
            .iter()
            .filter(|f| f.attrs.iter().any(|attr| attr.path().is_ident("callback")))
            .cloned()
            .collect::<Vec<_>>();

        let plain_fields = struct_
            .fields
            .iter()
            .filter(|f| {
                !f.attrs.iter().any(|attr| {
                    attr.path().is_ident("signal_prop")
                        || attr.path().is_ident("prop")
                        || attr.path().is_ident("callback")
                })
            })
            .cloned()
            .collect::<Vec<_>>();

        Self {
            ident: struct_.ident,
            signal_props: signal_props.into_iter().map(|v| v.into()).collect(),
            props: props.into_iter().map(|v| v.into()).collect(),
            plain_fields,
            callbacks: callbacks.into_iter().map(|v| v.into()).collect(),
        }
    }
}