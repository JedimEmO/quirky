use convert_case::{Case, Casing};
use proc_macro2::Ident;
use quote::quote;
use syn::{Expr, Field, Type, TypeParamBound};

#[derive(Clone)]
pub(crate) struct FnSignalProp {
    pub field_name: Ident,
    pub field_type: Type,
    pub signal_name: Ident,
    pub signal_type: Type,
    pub signal_fn_name: Ident,
    pub default: Option<Expr>,
}

impl From<Field> for FnSignalProp {
    fn from(f: Field) -> Self {
        let t = f.ty.clone();
        let ty = quote! {#t};
        let ident = f.ident.clone().expect("struct field parse error");

        let attrs = f.attrs.clone();

        let default = if let Some(d) = attrs.iter().find(|a| a.path().is_ident("default")) {
            let expr = d.parse_args().expect("field default parse error");
            Some(expr)
        } else {
            None
        };

        let signal_name = syn::parse_str::<Ident>(
            format!("{}Signal", ident)
                .as_str()
                .to_case(Case::Pascal)
                .as_str(),
        )
        .expect("failed to parse signal name");
        let signal_type = syn::parse_str::<Type>(
            format!("futures_signals::signal::Signal<Item={}>", ty).as_str(),
        )
        .expect("failed to parse signal type");
        let signal_fn_name =
            syn::parse_str::<Ident>(format!("{}SignalFn", ident).to_case(Case::Pascal).as_str())
                .expect("failed to parse signal fn name");

        FnSignalProp {
            field_name: ident,
            field_type: f.ty.clone(),
            signal_name,
            signal_type,
            signal_fn_name,
            default,
        }
    }
}

#[derive(Clone)]
pub(crate) struct SlotProp {
    pub slot_name: Ident,
    pub slot_type_name: Ident,
    pub slot_type: TypeParamBound,
    pub slot_default: Type,
}

impl From<Field> for SlotProp {
    fn from(f: Field) -> Self {
        let slot_name = f.ident.clone().expect("missing slot name");
        let slot_type_name = syn::parse_str::<Ident>(
            format!("{}Callback", slot_name)
                .to_case(Case::Pascal)
                .as_str(),
        )
        .expect("slot name parse fail");

        let msg_type = f.ty.clone();
        let slot_default =
            syn::parse_str::<Type>(format!("fn({}) -> ()", quote! {#msg_type}).as_str())
                .unwrap_or_else(|_| {
                    panic!("slot default parse error: fn({}) -> ()", quote! {#msg_type})
                });

        let slot_type =
            syn::parse_str::<TypeParamBound>(format!("Fn({}) -> ()", quote! {#msg_type}).as_str())
                .unwrap_or_else(|_| {
                    panic!("slot type parse error: Fn({}) -> ()", quote! {#msg_type})
                });

        SlotProp {
            slot_name,
            slot_type_name,
            slot_type,
            slot_default,
        }
    }
}
