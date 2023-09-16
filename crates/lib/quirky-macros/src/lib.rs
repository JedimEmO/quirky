use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use syn::{Expr, Ident, Type, TypeParamBound};

struct FnSignalField {
    pub field_name: Ident,
    pub field_type: Type,
    pub signal_name: Ident,
    pub signal_type: Type,
    pub signal_fn_name: Ident,
    pub default: Option<Expr>,
}

struct CallbackField {
    pub callback_name: Ident,
    pub callback_type_name: Ident,
    pub callback_type: TypeParamBound,
    pub callback_default: Type,
}

#[proc_macro_attribute]
pub fn widget(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    let struct_ = syn::parse::<syn::ItemStruct>(input).expect("failed to parse struct");
    let builder_name = syn::parse_str::<Ident>(format!("{}Builder", struct_.ident).as_str())
        .expect("builder name parse error");
    let struct_name = struct_.ident;

    let fields = struct_
        .fields
        .iter()
        .filter(|f| f.attrs.iter().any(|attr| attr.path().is_ident("signal")))
        .collect::<Vec<_>>();

    let callbacks = struct_
        .fields
        .iter()
        .filter(|f| f.attrs.iter().any(|attr| attr.path().is_ident("callback")))
        .collect::<Vec<_>>();

    let internal_fields = struct_
        .fields
        .iter()
        .filter(|f| !f.attrs.iter().any(|attr| attr.path().is_ident("signal")))
        .filter(|f| !f.attrs.iter().any(|attr| attr.path().is_ident("callback")))
        .collect::<Vec<_>>();

    let builder_struct_fields = fields
        .iter()
        .map(|f| {
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
            let signal_fn_name = syn::parse_str::<Ident>(
                format!("{}SignalFn", ident).to_case(Case::Pascal).as_str(),
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

    let builder_struct_callback_fields = callbacks
        .iter()
        .map(|f| {
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

            let callback_type = syn::parse_str::<TypeParamBound>(
                format!("Fn({}) -> ()", quote! {#msg_type}).as_str(),
            )
            .unwrap_or_else(|_| {
                panic!(
                    "callback type parse error: Fn({}) -> ()",
                    quote! {#msg_type}
                )
            });

            CallbackField {
                callback_name,
                callback_type_name,
                callback_type,
                callback_default,
            }
        })
        .collect::<Vec<_>>();

    let all_field_names = vec![
        builder_struct_fields
            .iter()
            .map(|f| f.field_name.clone())
            .collect::<Vec<_>>(),
        builder_struct_callback_fields
            .iter()
            .map(|f| f.callback_name.clone())
            .collect::<Vec<_>>(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    let builder_struct_signal_generics_params_struct = builder_struct_fields.iter().map(|f| {
        let FnSignalField { signal_name, signal_type, signal_fn_name, field_type, .. } = f;
        quote! { #signal_name: #signal_type + 'static = futures_signals::signal::Always<#field_type>, #signal_fn_name: Fn() -> #signal_name = fn() -> futures_signals::signal::Always<#field_type>}
    }).collect::<Vec<_>>();

    let builder_struct_callback_generics_params_struct = builder_struct_callback_fields
        .iter()
        .map(|f| {
            let CallbackField {
                callback_type_name,
                callback_type,
                callback_default,
                ..
            } = f;

            quote! { #callback_type_name: #callback_type + Send + Sync = #callback_default}
        })
        .collect::<Vec<_>>();

    let builder_struct_generics_params_struct = vec![
        builder_struct_signal_generics_params_struct,
        builder_struct_callback_generics_params_struct,
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

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

    let build_struct_callback_generics_params = builder_struct_callback_fields
        .iter()
        .map(|f| {
            let CallbackField {
                callback_type_name,
                callback_type,
                ..
            } = f;

            quote! { #callback_type_name: #callback_type + Send + Sync }
        })
        .collect::<Vec<_>>();

    let builder_struct_generics_params = vec![
        builder_struct_generics_params,
        build_struct_callback_generics_params,
    ]
    .into_iter()
    .flatten()
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

    let builder_struct_callback_generics_params_names = builder_struct_callback_fields
        .iter()
        .map(|f| {
            let CallbackField {
                callback_type_name, ..
            } = f;
            quote! { #callback_type_name }
        })
        .collect::<Vec<_>>();

    let builder_struct_generics_params_names = vec![
        builder_struct_generics_params_names,
        builder_struct_callback_generics_params_names,
    ]
    .into_iter()
    .flatten()
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

    let builder_struct_callback_members = builder_struct_callback_fields
        .iter()
        .map(|f| {
            let CallbackField {
                callback_name,
                callback_type_name,
                ..
            } = f;
            quote! { #callback_name: Option<#callback_type_name> }
        })
        .collect::<Vec<_>>();

    let builder_struct_members = vec![builder_struct_members, builder_struct_callback_members]
        .into_iter()
        .flatten();

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

    let builder_struct_callback_members_defaults = builder_struct_callback_fields
        .iter()
        .map(|f| {
            let CallbackField { callback_name, .. } = f;

            quote! { #callback_name: Some(|_| {}) }
        })
        .collect::<Vec<_>>();

    let builder_struct_members_defaults = vec![
        builder_struct_members_defaults,
        builder_struct_callback_members_defaults,
    ]
    .into_iter()
    .flatten();

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
    let real_struct_callback_members = builder_struct_callback_fields
        .iter()
        .map(|f| {
            let CallbackField {
                callback_name,
                callback_type_name,
                ..
            } = f;
            quote! { #callback_name: #callback_type_name }
        })
        .collect::<Vec<_>>();

    let real_struct_members = vec![real_struct_members, real_struct_callback_members]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    let real_struct_member_ctors = builder_struct_fields
        .iter()
        .map(|f| {
            let FnSignalField { field_name, .. } = f;
            quote! { #field_name: self.#field_name.expect("missing signal") }
        })
        .collect::<Vec<_>>();

    let real_struct_member_callback_ctors = builder_struct_callback_fields
        .iter()
        .map(|f| {
            let CallbackField {
                callback_name,
                callback_type_name: _,
                callback_type: _,
                callback_default: _,
            } = f;

            quote! { #callback_name: self.#callback_name.expect("missing callback")}
        })
        .collect::<Vec<_>>();

    let real_struct_member_ctors =
        vec![real_struct_member_ctors, real_struct_member_callback_ctors]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

    let builder_field_signal_setters = builder_struct_fields.iter().map(|f| {
        let FnSignalField { field_name, field_type, signal_name,  .. } = f;
        let fn_sig_name = syn::parse_str::<Ident>(format!("{}_signal", field_name).as_str()).expect("fn sig name parse error");

        let other_fields = all_field_names.iter().filter_map(|f| {
            if f == field_name {
                None
            } else {
                Some(quote! {#f: self.#f })
            }
        }).collect::<Vec<_>>();

        let builder_struct_generics_params_names_out = builder_struct_generics_params_names
            .iter()
            .map(|f| {
                if f.to_string().contains(&signal_name.to_string()) {
                    quote! { T, TFN }
                } else {
                    quote! { #f }
                }
            })
            .collect::<Vec<_>>();

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
        let FnSignalField { field_name, field_type, signal_name, signal_type: _, signal_fn_name: _, default: _, } = f;

        let other_fields = all_field_names.iter().filter_map(|f| {
            if f == field_name {
                None
            } else {
                Some(quote! {#f: self.#f })
            }
        }).collect::<Vec<_>>();

        let builder_struct_generics_params_names_out = builder_struct_generics_params_names
            .iter()
            .map(|f| {
                if f.to_string().contains(&signal_name.to_string()) {
                    quote! { futures_signals::signal::Always<#field_type>, Box<dyn Fn() -> futures_signals::signal::Always<#field_type> + Send + Sync>  }
                } else {
                    quote! { #f }
                }
            })
            .collect::<Vec<_>>();

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

    let builder_field_callback_setters = builder_struct_callback_fields.iter().map(|f| {
        let CallbackField { callback_name, callback_type, callback_type_name, .. } = f;

        let fn_name = callback_name;

        let builder_struct_generics_params_names_out = builder_struct_generics_params_names
            .iter()
            .map(|f| {
                if callback_type_name == f.to_string().as_str() {
                    quote! { T }
                } else {
                    quote! { #f }
                }
            })
            .collect::<Vec<_>>();

        let other_fields = all_field_names.iter().filter_map(|f| {
            if f == callback_name {
                None
            } else {
                Some(quote! {#f: self.#f })
            }
        }).collect::<Vec<_>>();

        quote! {
            impl<#(#builder_struct_generics_params),*> #builder_name<#(#builder_struct_generics_params_names),*> {
                pub fn #fn_name<T: #callback_type + Send + Sync>(self, value: T) -> #builder_name<#(#builder_struct_generics_params_names_out),*> {
                    #builder_name {
                        #callback_name: Some(value),
                        #(#other_fields),*
                    }
                }
            }
        }
    }).collect::<Vec<_>>();

    let struct_fields_decl = internal_fields
        .iter()
        .map(|f| {
            let ident = f.ident.clone().expect("struct field decl parse error");
            let ty = f.ty.clone();

            quote! { #ident: #ty }
        })
        .collect::<Vec<_>>();

    let struct_fields_init = internal_fields
        .iter()
        .map(|f| {
            let ident = f.ident.clone().expect("struct field init parse error");

            let attrs = f.attrs.clone();
            let visibility = f.vis.clone();

            if let Some(d) = attrs.iter().find(|a| a.path().is_ident("default")) {
                let expr: Expr = d
                    .parse_args()
                    .expect("struct field init default parse error");

                quote! { #visibility #ident: #expr }
            } else {
                quote! { #visibility #ident: Default::default() }
            }
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
        #(#builder_field_callback_setters)*

        impl<#(#builder_struct_generics_params),*> #builder_name<#(#builder_struct_generics_params_names),*> {
            pub fn build(self) -> Arc<#struct_name<#(#builder_struct_generics_params_names),*>> {
                Arc::new(#struct_name {
                    id: uuid::Uuid::new_v4(),
                    bounding_box: Default::default(),
                    dirty: Default::default(),
                    #(#real_struct_member_ctors),*,
                    #(#struct_fields_init),*
                })
            }
        }

        pub struct #struct_name<#(#builder_struct_generics_params),*> {
            id: uuid::Uuid,
            bounding_box: futures_signals::signal::Mutable<LayoutBox>,
            dirty: futures_signals::signal::Mutable<bool>,
            #(#real_struct_members),*,
            #(#struct_fields_decl),*
        }

        impl<#(#builder_struct_generics_params),*> WidgetBase for #struct_name<#(#builder_struct_generics_params_names),*> {
            fn id(&self) -> Uuid {
                self.id
            }

             fn set_bounding_box(&self, new_box: LayoutBox) {
                self.bounding_box.set(new_box);
            }

            fn bounding_box(&self) -> futures_signals::signal::ReadOnlyMutable<LayoutBox> {
                self.bounding_box.read_only()
            }

            fn dirty(&self)  -> futures_signals::signal::ReadOnlyMutable<bool> {
                self.dirty.read_only()
            }

            fn set_dirty(&self) -> () {
                self.dirty.set(true);
            }

            fn clear_dirty(&self) -> () {
                self.dirty.set(false);
            }
        }
    }.into()
}
