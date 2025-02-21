use proc_macro::TokenStream;
use proc_macro2::{TokenStream as TStream2, TokenTree};
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, Attribute, Data, DeriveInput, Fields, GenericParam, Generics,
    Path, Type,
};

fn krate() -> TStream2 {
    quote!(::tokio_telegraf)
}

#[proc_macro_derive(Metric, attributes(measurement, telegraf))]
pub fn derive_metric(tokens: TokenStream) -> TokenStream {
    expand_metric(tokens)
}

fn expand_metric(tokens: TokenStream) -> TokenStream {
    let krate = krate();
    let input = parse_macro_input!(tokens as DeriveInput);

    let name = &input.ident;
    let measurement = get_measurement_name(&input);

    let generics = add_trait_bounds(input.generics);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let pt = get_to_point(&input.data);

    let expanded = quote! {
        impl #impl_generics #krate::Metric for #name #ty_generics #where_clause {
            fn to_point(&self) -> #krate::Point {
                let mut pf: Vec<(String, Box<dyn #krate::IntoFieldData>)> = Vec::new();
                let mut pt: Vec<(String, String)> = Vec::new();
                let mut tsp: Option<u64> = None;
                #pt
                #krate::Point::new(#measurement, pt, pf, tsp)
            }
        }
    };

    TokenStream::from(expanded)
}

fn get_measurement_name(input: &DeriveInput) -> TStream2 {
    let default = &input.ident;
    let measurement = input
        .attrs
        .iter()
        .find(|a| a.path.segments.len() == 1 && a.path.segments[0].ident == "measurement");

    match measurement {
        Some(attr) => {
            let q = attr
                .tokens
                .clone()
                .into_iter()
                .nth(1)
                .map(|t| match t {
                    TokenTree::Literal(l) => l,
                    _ => panic!("unexpected type"),
                })
                .unwrap();
            quote!(#q.to_string())
        }
        None => quote!(stringify!(#default).to_string()),
    }
}

fn add_trait_bounds(mut generics: Generics) -> Generics {
    let krate = krate();
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(#krate::Metric));
        }
    }
    generics
}

fn has_attr(attr: &Attribute) -> bool {
    attr.path
        .segments
        .iter()
        .last()
        .map(|seg| seg.ident.to_string())
        .unwrap_or_default()
        == "telegraf"
}

fn check_attr(t_tree: TokenTree, cmp: &str) -> bool {
    match t_tree {
        TokenTree::Group(group) => group
            .stream()
            .into_iter()
            .next()
            .map(|token_tree| match token_tree {
                TokenTree::Ident(ident) => ident == cmp,
                _ => false,
            })
            .unwrap(),
        _ => false,
    }
}

fn is_tag(attr: &Attribute) -> bool {
    if !has_attr(attr) {
        return false;
    }

    attr.tokens
        .clone()
        .into_iter()
        .next()
        .map(|t_tree| check_attr(t_tree, "tag"))
        .unwrap()
}

fn is_timestamp(attr: &Attribute) -> bool {
    if !has_attr(attr) {
        return false;
    }

    attr.tokens
        .clone()
        .into_iter()
        .next()
        .map(|t_tree| check_attr(t_tree, "timestamp"))
        .unwrap()
}

fn get_to_point(data: &Data) -> TStream2 {
    fn path_is_option(path: &Path) -> bool {
        path.leading_colon.is_none()
            && path.segments.len() == 1
            && path.segments.iter().next().unwrap().ident == "Option"
    }

    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    fields.named
                        .iter()
                        .map(|f| {
                            match &f.ty {
                                Type::Path(typath) if typath.qself.is_none() && path_is_option(&typath.path) => {
                                    let name = &f.ident;
                                    if f.attrs.iter().any(is_tag) {
                                        quote!(
                                            if let Some(ref v) = self.#name {
                                                pt.push((stringify!(#name).to_string(), format!("{}", v)));
                                            }
                                        )
                                    } else if f.attrs.iter().any(is_timestamp) {
                                        quote!(
                                            if let Some(ref v) = self.#name {
                                                tsp = tsp.or(Some(v.into()));
                                            }
                                        )
                                    } else {
                                        quote!(
                                            if let Some(ref v) = self.#name {
                                                pf.push((stringify!(#name).to_string(), Box::new(v.clone())));
                                            }
                                        )
                                    }
                                },
                                _ => {
                                    let name = &f.ident;
                                    if f.attrs.iter().any(is_tag) {
                                        quote!(pt.push((stringify!(#name).to_string(), format!("{}", self.#name)));)
                                    } else if f.attrs.iter().any(is_timestamp) {
                                        quote!(tsp = tsp.or(Some(self.#name.into()));)
                                    } else {
                                        quote!(pf.push((stringify!(#name).to_string(), Box::new(self.#name.clone())));)
                                    }
                                }
                            }
                        })
                        .collect()
                }
                _ => panic!("only named fields supported")
            }
        }
        _ => panic!("cannot derive for data type")
    }
}
