use proc_macro::TokenStream;
use std::thread::scope;
use convert_case::{Case, Casing};
use proc_macro2::Ident;
use quote::quote;
use syn::{ItemStruct, Type, TypePath};

#[proc_macro_attribute]
pub fn widget(args_stream: TokenStream, token_stream: TokenStream) -> TokenStream {
    let struct_ = syn::parse::<syn::ItemStruct>(token_stream).expect("failed to parse struct");
    // let arg = syn::parse::<AttributeArgument>(args).expect("failed to parse attribute args");

    let props = parse_widget_props(&struct_);
    let props = render_widget_props_struct(&props);

    quote! {
        #props
    }.into()
}

struct WidgetField {
    pub allow_static: bool,
    pub ty_: Type,
}

struct WidgetProps {
    pub name: Ident,
    pub fields: Vec<WidgetField>,
}

fn parse_widget_props(strct: &ItemStruct) -> WidgetProps {
    let fields = strct.fields.iter().map(|f| {
        WidgetField { allow_static: true, ty_: f.ty.clone() }
    }).collect();

    let name = strct.ident.clone();

    WidgetProps {
        name,
        fields,
    }
}

fn render_widget_props_struct(props: &WidgetProps) -> proc_macro2::TokenStream {
    let props_name = syn::parse_str::<Ident>(format!("{}Props", props.name).to_case(Case::Pascal).as_str()).unwrap();
    quote! {
        pub struct #props_name {}

        impl #props_name {
            pub fn new() -> Self {
                Self {}
            }
        }
    }
}