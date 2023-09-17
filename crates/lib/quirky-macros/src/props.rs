use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::spanned::Spanned;
use syn::{Expr, Field, Type, TypeParamBound};

#[derive(Clone)]
pub(crate) struct Prop {
    pub field_name: Ident,
    pub field_type: Type,
    pub default: Option<Expr>,
    pub span: Span,
}

impl From<Field> for Prop {
    fn from(f: Field) -> Self {
        let attrs = f.attrs.clone();

        let default = if let Some(d) = attrs.iter().find(|a| a.path().is_ident("default")) {
            let expr = d.parse_args().expect("field default parse error");
            Some(expr)
        } else {
            None
        };

        Self {
            field_name: f.ident.clone().expect("prop missing ident"),
            field_type: f.ty.clone(),
            default,
            span: f.span().clone(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct FnSignalProp {
    pub field_name: Ident,
    pub field_type: Type,
    pub signal_name: Ident,
    pub signal_type: Type,
    pub signal_fn_name: Ident,
    pub default: Option<Expr>,
    pub span: Span,
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
            span: f.span(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct CallbackProp {
    pub callback_name: Ident,
    pub callback_type_name: Ident,
    pub callback_type: TypeParamBound,
    pub callback_default: Type,
}

impl From<Field> for CallbackProp {
    fn from(f: Field) -> Self {
        let callback_name = f.ident.clone().expect("missing callback name");
        let callback_type_name = syn::parse_str::<Ident>(
            format!("{}Callback", callback_name)
                .to_case(Case::Pascal)
                .as_str(),
        )
        .expect("callback name parse fail");

        let msg_type = f.ty.clone();
        let callback_default =
            syn::parse_str::<Type>(format!("fn({}) -> ()", quote! {#msg_type}).as_str())
                .unwrap_or_else(|_| {
                    panic!(
                        "callback default parse error: fn({}) -> ()",
                        quote! {#msg_type}
                    )
                });

        let callback_type =
            syn::parse_str::<TypeParamBound>(format!("Fn({}) -> ()", quote! {#msg_type}).as_str())
                .unwrap_or_else(|_| {
                    panic!(
                        "callback type parse error: Fn({}) -> ()",
                        quote! {#msg_type}
                    )
                });

        CallbackProp {
            callback_name,
            callback_type_name,
            callback_type,
            callback_default,
        }
    }
}
