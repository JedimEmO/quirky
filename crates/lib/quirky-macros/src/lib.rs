use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use syn::{Expr, Ident, Type};

struct FnSignalField {
    pub field_name: Ident,
    pub field_type: Type,
    pub signal_name: Ident,
    pub signal_type: Type,
    pub signal_fn_name: Ident,
    pub default: Option<Expr>,
}

#[proc_macro_attribute]
pub fn widget(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    let struct_ = syn::parse::<syn::ItemStruct>(input).expect("failed to parse struct");
    let builder_name =
        syn::parse_str::<Ident>(format!("{}Builder", struct_.ident).as_str()).unwrap();
    let struct_name = struct_.ident;

    let fields = struct_
        .fields
        .iter()
        .filter(|f| f.attrs.iter().any(|attr| attr.path().is_ident("signal")))
        .collect::<Vec<_>>();

    let internal_fields = struct_
        .fields
        .iter()
        .filter(|f| !f.attrs.iter().any(|attr| attr.path().is_ident("signal")))
        .collect::<Vec<_>>();

    let builder_struct_fields = fields
        .iter()
        .map(|f| {
            let t = f.ty.clone();
            let ty = quote! {#t};
            let ident = f.ident.clone().unwrap();

            let attrs = f.attrs.clone();

            let default = if let Some(d) = attrs.iter().find(|a| a.path().is_ident("default")) {
                let expr = d.parse_args().unwrap();
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
            let signal_fn_name = syn::parse_str::<Ident>(
                format!("{}SignalFn", ident)
                    .as_str()
                    .to_case(Case::Pascal)
                    .as_str(),
            )
            .expect("failed to parse signal fn name");

            FnSignalField {
                field_name: ident,
                field_type: f.ty.clone(),
                signal_name,
                signal_type,
                signal_fn_name,
                default,
            }
        })
        .collect::<Vec<_>>();

    let builder_struct_generics_params_struct = builder_struct_fields.iter().map(|f| {
        let FnSignalField { signal_name, signal_type, signal_fn_name, field_type, .. } = f;
        quote! { #signal_name: #signal_type + 'static = futures_signals::signal::Always<#field_type>, #signal_fn_name: Fn() -> #signal_name = fn() -> futures_signals::signal::Always<#field_type>}
    }).collect::<Vec<_>>();

    let builder_struct_generics_params = builder_struct_fields
        .iter()
        .map(|f| {
            let FnSignalField {
                signal_name,
                signal_type,
                signal_fn_name,
                ..
            } = f;
            quote! { #signal_name: #signal_type + Send + Sync + Unpin + 'static, #signal_fn_name: Fn() -> #signal_name + Send + Sync + 'static }
        })
        .collect::<Vec<_>>();

    let builder_struct_generics_params_names = builder_struct_fields
        .iter()
        .map(|f| {
            let FnSignalField {
                signal_name,
                signal_fn_name,
                ..
            } = f;
            quote! { #signal_name, #signal_fn_name }
        })
        .collect::<Vec<_>>();

    let builder_struct_members = builder_struct_fields
        .iter()
        .map(|f| {
            let FnSignalField {
                field_name,
                signal_fn_name,
                ..
            } = f;
            quote! { #field_name: Option<#signal_fn_name> }
        })
        .collect::<Vec<_>>();

    let builder_struct_members_defaults = builder_struct_fields
        .iter()
        .map(|f| {
            let FnSignalField {
                field_name,
                default,
                ..
            } = f;

            if let Some(default) = default {
                quote! { #field_name: Some(|| futures_signals::signal::always(#default)) }
            } else {
                quote! { #field_name: None }
            }
        })
        .collect::<Vec<_>>();

    let real_struct_members = builder_struct_fields
        .iter()
        .map(|f| {
            let FnSignalField {
                field_name,
                signal_fn_name,
                ..
            } = f;
            quote! { #field_name: #signal_fn_name }
        })
        .collect::<Vec<_>>();

    let real_struct_member_ctors = builder_struct_fields
        .iter()
        .map(|f| {
            let FnSignalField { field_name, .. } = f;
            quote! { #field_name: self.#field_name.expect("missing signal") }
        })
        .collect::<Vec<_>>();

    let builder_field_signal_setters = builder_struct_fields.iter().map(|f| {
        let FnSignalField { field_name, field_type, signal_name, .. } = f;
        let fn_sig_name = syn::parse_str::<Ident>(format!("{}_signal", field_name).as_str()).unwrap();

        let builder_struct_generics_params_names_out = builder_struct_fields
            .iter()
            .map(|f| {
                let FnSignalField {
                    signal_name: sn2,
                    signal_fn_name: sfn2,
                    ..
                } = f;

                if signal_name == sn2 {
                    quote! { T, TFN }
                } else {
                    quote! { #sn2, #sfn2 }
                }
            })
            .collect::<Vec<_>>();

        let other_fields = builder_struct_fields.iter().filter_map(|f| {
            if &f.field_name == field_name {
                None
            } else {
                let name = f.field_name.clone();

                Some(quote! {#name: self.#name })
            }
        }).collect::<Vec<_>>();


        quote! {
            impl<#(#builder_struct_generics_params),*> #builder_name<#(#builder_struct_generics_params_names),*> {
                pub fn #fn_sig_name<T: futures_signals::signal::Signal<Item=#field_type> + Sync + Send + Unpin, TFN: Fn() -> T>(self, value: TFN) -> #builder_name<#(#builder_struct_generics_params_names_out),*> {
                    #builder_name {
                        #field_name: Some(value),
                        #(#other_fields),*
                    }
                }
            }
        }
    }).collect::<Vec<_>>();

    let builder_field_value_setters = builder_struct_fields.iter().map(|f| {
        let FnSignalField { field_name, field_type, signal_name, .. } = f;

        let other_fields = builder_struct_fields.iter().filter_map(|f| {
            if &f.field_name == field_name {
                None
            } else {
                let name = f.field_name.clone();

                Some(quote! {#name: self.#name })
            }
        }).collect::<Vec<_>>();

        let builder_struct_generics_params_names_out = builder_struct_fields.iter().map(|f| {
            let FnSignalField { signal_name: sn, signal_fn_name: sfn, .. } = f;

            if sn == signal_name {
                quote! { futures_signals::signal::Always<#field_type>, Box<dyn Fn() -> futures_signals::signal::Always<#field_type> + Send + Sync> }
            } else {
                quote! { #sn, #sfn }
            }
        }).collect::<Vec<_>>();

        quote! {
            impl<#(#builder_struct_generics_params),*> #builder_name<#(#builder_struct_generics_params_names),*> {
                pub fn #field_name(self, value: #field_type) -> #builder_name<#(#builder_struct_generics_params_names_out),*> {
                    #builder_name {
                        #field_name: Some(Box::new(move || futures_signals::signal::always(value.clone()))),
                        #(#other_fields),*
                    }
                }
            }
        }
    }).collect::<Vec<_>>();

    let struct_fields_decl = internal_fields
        .iter()
        .map(|f| {
            let ident = f.ident.clone().unwrap();
            let ty = f.ty.clone();

            quote! { #ident: #ty }
        })
        .collect::<Vec<_>>();

    let struct_fields_init = internal_fields
        .iter()
        .map(|f| {
            let ident = f.ident.clone().unwrap();

            quote! { #ident: Default::default() }
        })
        .collect::<Vec<_>>();

    quote! {
        pub struct #builder_name<#(#builder_struct_generics_params_struct),*> {
            #(#builder_struct_members),*
        }

        impl #builder_name {
            pub fn new() -> Self {
                Self {
                    #(#builder_struct_members_defaults),*
                }
            }
        }
        #(#builder_field_signal_setters)*

        #(#builder_field_value_setters)*

        impl<#(#builder_struct_generics_params),*> #builder_name<#(#builder_struct_generics_params_names),*> {
            pub fn build(self) -> Arc<#struct_name<#(#builder_struct_generics_params_names),*>> {
                Arc::new(#struct_name {
                    id: uuid::Uuid::new_v4(),
                    bounding_box: Default::default(),
                    #(#real_struct_member_ctors),*,
                    #(#struct_fields_init),*
                })
            }
        }

        pub struct #struct_name<#(#builder_struct_generics_params),*> {
            id: uuid::Uuid,
            bounding_box: futures_signals::signal::Mutable<LayoutBox>,
            #(#real_struct_members),*,
            #(#struct_fields_decl),*
        }
    }.into()
}
