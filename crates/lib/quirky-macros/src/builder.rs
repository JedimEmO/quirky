use crate::props::{FnSignalProp, SlotProp};
use crate::widget_struct::WidgetStructParsed;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::Expr;

/// Describes the builder for our widget struct
/// Contains all the type-changing setters for our signal props and slots.
/// Setting either of these will result in a new generic parameter type to the builder struct,
/// so they are fairly complicated.
pub(crate) struct BuilderStruct {
    pub widget_struct: WidgetStructParsed,
}

impl BuilderStruct {
    /// All the member of the builder, for holding intermediate values
    /// i.e
    /// ```rust,ignore
    /// struct MyBuilder<...> {
    ///   {..member_fields()}
    /// }
    /// ```
    pub fn member_fields(&self) -> Vec<TokenStream> {
        let builder_struct_members = self
            .widget_struct
            .signal_props
            .iter()
            .map(|f| {
                let FnSignalProp {
                    field_name,
                    signal_fn_name,
                    ..
                } = f;
                quote! { #field_name: Option<#signal_fn_name> }
            })
            .collect::<Vec<_>>();

        let builder_struct_slot_members = self
            .widget_struct
            .slots
            .iter()
            .map(|f| {
                let SlotProp {
                    slot_name,
                    slot_type_name,
                    ..
                } = f;
                quote! { #slot_name: Option<#slot_type_name> }
            })
            .collect::<Vec<_>>();

        vec![builder_struct_members, builder_struct_slot_members]
            .into_iter()
            .flatten()
            .collect()
    }

    pub fn member_field_names(&self) -> Vec<Ident> {
        let signal_field_names = self
            .widget_struct
            .signal_props
            .iter()
            .map(|f| f.field_name.clone())
            .collect::<Vec<_>>();
        let slot_field_names = self
            .widget_struct
            .slots
            .iter()
            .map(|f| f.slot_name.clone())
            .collect::<Vec<_>>();

        vec![signal_field_names, slot_field_names]
            .into_iter()
            .flatten()
            .collect()
    }

    /// The initialization of all the builder members, i.e.
    /// ```rust,ignore
    /// impl MyBuilder {
    ///     fn new() -> Self {
    ///         Self {
    ///             { ..memer_defaults() }
    ///         }
    ///     }
    /// }
    /// ```
    fn member_defaults(&self) -> Vec<TokenStream> {
        let builder_struct_members_defaults = self
            .widget_struct
            .signal_props
            .iter()
            .map(|f| {
                let FnSignalProp {
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

        let builder_struct_slot_members_defaults = self
            .widget_struct
            .slots
            .iter()
            .map(|f| {
                let SlotProp { slot_name, .. } = f;

                quote! { #slot_name: Some(|_| {}) }
            })
            .collect::<Vec<_>>();

        vec![
            builder_struct_members_defaults,
            builder_struct_slot_members_defaults,
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// All the generic parameters for the builder struct declaration, which includes default types, i.e.
    /// ```rust,ignore
    /// MyBuilder<{all_generic_params_struct_decl}> { ... }
    /// ```
    pub fn all_generic_params_struct_decl(&self) -> Vec<TokenStream> {
        let signal_params = self.widget_struct.signal_props
            .iter()
            .map(|f| {
                let FnSignalProp { signal_name, signal_type, signal_fn_name, field_type, .. } = f;
                quote! { #signal_name: #signal_type + 'static = futures_signals::signal::Always<#field_type>, #signal_fn_name: Fn() -> #signal_name = fn() -> futures_signals::signal::Always<#field_type>}
            }).collect::<Vec<_>>();

        let slot_params = self
            .widget_struct
            .slots
            .iter()
            .map(|f| {
                let SlotProp {
                    slot_type_name,
                    slot_type,
                    slot_default,
                    ..
                } = f;

                quote! { #slot_type_name: #slot_type + Send + Sync = #slot_default}
            })
            .collect::<Vec<_>>();

        vec![signal_params, slot_params]
            .into_iter()
            .flatten()
            .collect()
    }

    /// All the generic parameters for the builder struct, i.e.
    /// ```rust,ignore
    /// impl<{all_generic_params()}> MyBuilder<...> { ... }
    /// ```
    pub fn all_generic_params(&self) -> Vec<TokenStream> {
        let signal_params = self.widget_struct.signal_props.iter()
            .map(|f| {
                let FnSignalProp {
                    signal_name,
                    signal_type,
                    signal_fn_name,
                    ..
                } = f;
                quote! { #signal_name: #signal_type + Send + Sync + Unpin + 'static, #signal_fn_name: Fn() -> #signal_name + Send + Sync + 'static }
            })
            .collect::<Vec<_>>();

        let slot_params = self
            .widget_struct
            .slots
            .iter()
            .map(|f| {
                let SlotProp {
                    slot_type_name,
                    slot_type,
                    ..
                } = f;

                quote! { #slot_type_name: #slot_type + Send + Sync + 'static }
            })
            .collect::<Vec<_>>();

        vec![signal_params, slot_params]
            .into_iter()
            .flatten()
            .collect()
    }

    /// The names of all the generic params, i.e.
    /// ```rust,ignore
    /// let foo: MyBuilder<{all_generic_params_names}> = ...;
    /// ```
    pub fn all_generic_params_names(&self) -> Vec<TokenStream> {
        let signal_param_names = self
            .widget_struct
            .signal_props
            .iter()
            .map(|f| {
                let FnSignalProp {
                    signal_name,
                    signal_fn_name,
                    ..
                } = f;
                quote! { #signal_name, #signal_fn_name }
            })
            .collect::<Vec<_>>();

        let slot_param_names = self
            .widget_struct
            .slots
            .iter()
            .map(|f| {
                let SlotProp { slot_type_name, .. } = f;
                quote! { #slot_type_name }
            })
            .collect::<Vec<_>>();

        vec![signal_param_names, slot_param_names]
            .into_iter()
            .flatten()
            .collect()
    }

    /// All the type changing setter functions.
    /// These will transform the template types of the builder for their field
    ///
    /// ```rust,ignore
    /// impl Builder<T, U, F> {
    ///   fn set_t_signal<TN, UN>(&self, t: TN, u: UN) -> Builder<TN, UN, F> { ...}
    ///   fn set_f<FN>(f: FN) -> Builder<T, U, Fn> {...}
    /// }
    /// ```
    pub fn setter_functions(&self) -> Vec<TokenStream> {
        let all_member_names = self.member_field_names();
        let builder_struct_generics_params_names = self.all_generic_params_names();
        let builder_struct_generics_params = self.all_generic_params();
        let builder_name = self.builder_name();

        let builder_field_signal_setters = self.widget_struct.signal_props.iter().map(|f| {
            let FnSignalProp { field_name, field_type, signal_name,  .. } = f;
            let fn_sig_name = syn::parse_str::<Ident>(format!("{}_signal", field_name).as_str()).expect("fn sig name parse error");

            let other_fields = all_member_names.iter().filter_map(|f| {
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

        let builder_field_value_setters = self.widget_struct.signal_props.iter().map(|f| {
            let FnSignalProp { field_name, field_type, signal_name, signal_type: _, signal_fn_name: _, default: _, ..} = f;

            let other_fields = all_member_names.iter().filter_map(|f| {
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

        let builder_field_slot_setters = self.widget_struct.slots.iter().map(|f| {
            let SlotProp { slot_name, slot_type, slot_type_name, .. } = f;

            let fn_name = slot_name;

            let builder_struct_generics_params_names_out = builder_struct_generics_params_names
                .iter()
                .map(|f| {
                    if slot_type_name == f.to_string().as_str() {
                        quote! { T }
                    } else {
                        quote! { #f }
                    }
                })
                .collect::<Vec<_>>();

            let other_fields = all_member_names.iter().filter_map(|f| {
                if f == slot_name {
                    None
                } else {
                    Some(quote! {#f: self.#f })
                }
            }).collect::<Vec<_>>();

            quote! {
            impl<#(#builder_struct_generics_params),*> #builder_name<#(#builder_struct_generics_params_names),*> {
                pub fn #fn_name<T: #slot_type + Send + Sync>(self, value: T) -> #builder_name<#(#builder_struct_generics_params_names_out),*> {
                    #builder_name {
                        #slot_name: Some(value),
                        #(#other_fields),*
                    }
                }
            }
        }
        }).collect::<Vec<_>>();

        vec![
            builder_field_signal_setters,
            builder_field_value_setters,
            builder_field_slot_setters,
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    fn real_struct_props(&self) -> Vec<TokenStream> {
        let signal_prop_value_members = self
            .widget_struct
            .signal_props
            .iter()
            .map(|f| {
                let FnSignalProp {
                    field_name,
                    field_type,
                    ..
                } = f;
                let name = syn::parse_str::<Ident>(format!("{}_prop_value", field_name).as_str())
                    .expect("f");
                quote! { #name: futures_signals::signal::Mutable<Option<#field_type>>}
            })
            .collect::<Vec<_>>();

        let real_struct_members = self
            .widget_struct
            .signal_props
            .iter()
            .map(|f| {
                let FnSignalProp {
                    field_name,
                    signal_fn_name,
                    ..
                } = f;
                quote! { #field_name: #signal_fn_name }
            })
            .collect::<Vec<_>>();

        let real_struct_slot_members = self
            .widget_struct
            .slots
            .iter()
            .map(|f| {
                let SlotProp {
                    slot_name,
                    slot_type_name,
                    ..
                } = f;
                quote! { #slot_name: #slot_type_name }
            })
            .collect::<Vec<_>>();

        let struct_fields_decl = self
            .widget_struct
            .plain_fields
            .iter()
            .map(|f| {
                let ident = f.ident.clone().expect("struct field decl parse error");
                let ty = f.ty.clone();

                quote! { #ident: #ty }
            })
            .collect::<Vec<_>>();

        vec![
            real_struct_members,
            real_struct_slot_members,
            struct_fields_decl,
            signal_prop_value_members,
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
    }

    fn real_struct_props_init(&self) -> Vec<TokenStream> {
        let signal_prop_value_members_ctor = self
            .widget_struct
            .signal_props
            .iter()
            .map(|f| {
                let FnSignalProp {
                    field_name,
                    default,
                    ..
                } = f;

                let name = syn::parse_str::<Ident>(format!("{}_prop_value", field_name).as_str())
                    .expect("f");

                if let Some(default) = default {
                    quote! { #name: futures_signals::signal::Mutable::new(Some(#default)) }
                } else {
                    quote! { #name: futures_signals::signal::Mutable::new(None) }
                }
            })
            .collect::<Vec<_>>();

        let real_struct_member_ctors = self
            .widget_struct
            .signal_props
            .iter()
            .map(|f| {
                let FnSignalProp { field_name, .. } = f;
                quote! { #field_name: self.#field_name.expect("missing signal") }
            })
            .collect::<Vec<_>>();

        let real_struct_member_slot_ctors = self
            .widget_struct
            .slots
            .iter()
            .map(|f| {
                let SlotProp {
                    slot_name,
                    slot_type_name: _,
                    slot_type: _,
                    slot_default: _,
                } = f;

                quote! { #slot_name: self.#slot_name.expect("missing slot")}
            })
            .collect::<Vec<_>>();

        let struct_fields_init = self
            .widget_struct
            .plain_fields
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

        vec![
            real_struct_member_ctors,
            real_struct_member_slot_ctors,
            struct_fields_init,
            signal_prop_value_members_ctor,
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
    }

    /// Generates a runner function for all the token streams,
    /// to avoid lots and lots of boilerplate to populate field value props with stream values
    fn props_runner(&self) -> TokenStream {
        let sig_gens = self.widget_struct.signal_props.iter().map(|s| {
            let runner = self.sig_runner(s);

            quote! {
                #runner.boxed()
            }
        });

        quote! {
            fn poll_prop_futures<'a>(&'a self, ctx: &'a quirky::quirky_app_context::QuirkyAppContext) -> futures::stream::FuturesUnordered<futures::future::BoxFuture<'a, ()>> {
                let mut futs = futures::stream::FuturesUnordered::new();

                for f in vec![#(#sig_gens),*] {
                    futs.push(f);
                }

                futs.push(self
                    .bounding_box
                    .signal()
                    .throttle(|| async_std::task::sleep(std::time::Duration::from_millis(20)))
                    .for_each(|_| {
                        let ctx = &*ctx;
                        self.set_dirty();

                        async move {
                            ctx.signal_redraw().await;
                        }
                    }).boxed());

                futs
            }
        }
    }

    fn sig_runner(&self, sig: &FnSignalProp) -> TokenStream {
        let sig_name = sig.field_name.clone();
        let sig_propname =
            syn::parse_str::<Ident>(format!("{}_prop_value", sig_name).as_str()).expect("f");

        quote! {
            (self.#sig_name)().for_each(|incoming_value| {
                self.#sig_propname.set(Some(incoming_value));
                let ctx = &*ctx;
                self.set_dirty();
                async move {
                    ctx.signal_redraw().await;
                }
            })
        }
    }

    fn builder_name(&self) -> Ident {
        syn::parse_str::<Ident>(format!("{}Builder", self.widget_struct.ident).as_str())
            .expect("builder name parse error")
    }

    fn struct_name(&self) -> Ident {
        self.widget_struct.ident.clone()
    }
}

impl Into<proc_macro::TokenStream> for BuilderStruct {
    fn into(self) -> proc_macro::TokenStream {
        let builder_name = self.builder_name();
        let struct_name = self.struct_name();
        let builder_struct_generics_params = self.all_generic_params();
        let builder_struct_generics_params_decl = self.all_generic_params_struct_decl();
        let builder_struct_members = self.member_fields();
        let builder_struct_members_defaults = self.member_defaults();
        let builder_struct_generics_params_names = self.all_generic_params_names();
        let field_setter = self.setter_functions();

        let real_struct_members = self.real_struct_props();
        let real_struct_member_inits = self.real_struct_props_init();

        let props_runner = self.props_runner();

        quote! {
        pub struct #builder_name<#(#builder_struct_generics_params_decl),*> {
            #(#builder_struct_members),*
        }

        impl #builder_name {
            pub fn new() -> Self {
                Self {
                    #(#builder_struct_members_defaults),*
                }
            }
        }

        #(#field_setter)*

        impl<#(#builder_struct_generics_params),*> #builder_name<#(#builder_struct_generics_params_names),*> {
            pub fn build(self) -> std::sync::Arc<dyn Widget + 'static> {
                #struct_name {
                    id: uuid::Uuid::new_v4(),
                    bounding_box: Default::default(),
                    dirty: Default::default(),
                    #(#real_struct_member_inits),*
                }.build()
            }
        }

        pub struct #struct_name<#(#builder_struct_generics_params),*> {
            id: uuid::Uuid,
            bounding_box: futures_signals::signal::Mutable<quirky::LayoutBox>,
            dirty: futures_signals::signal::Mutable<bool>,
            #(#real_struct_members),*
        }

        impl<#(#builder_struct_generics_params),*> #struct_name<#(#builder_struct_generics_params_names),*> {

        }

        impl<#(#builder_struct_generics_params),*> quirky::widget::WidgetBase for #struct_name<#(#builder_struct_generics_params_names),*> {
            fn id(&self) -> uuid::Uuid {
                self.id
            }

             fn set_bounding_box(&self, new_box: quirky::LayoutBox) {
                self.bounding_box.set(new_box);
            }

            fn bounding_box(&self) -> futures_signals::signal::ReadOnlyMutable<quirky::LayoutBox> {
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

            #props_runner
        }
    }.into()
    }
}
