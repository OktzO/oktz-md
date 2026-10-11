use proc_macro::TokenStream;
use quote::quote;
use syn::{Expr, ExprLit, ItemFn, Lit, Meta};

/// `#[plugin(name="menu", alias=["m","help"], category="main", cooldown=5, ...)]`
///
/// Melekatkan fungsi sebagai plugin: membuat `static` `PluginEntry` dan
/// mendaftarkannya lewat `oktz_plugin_api::registry::__register`.
#[proc_macro_attribute]
pub fn plugin(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr2 = proc_macro2::TokenStream::from(attr);
    let parser = syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated;
    let args = match syn::parse::Parser::parse2(parser, attr2) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error().into(),
    };
    let func: ItemFn = match syn::parse(item) {
        Ok(f) => f,
        Err(e) => return e.to_compile_error().into(),
    };

    let mut name = String::new();
    let mut alias: Vec<String> = Vec::new();
    let mut category = String::new();
    let mut description = String::new();
    let mut usage = String::new();
    let mut example = String::new();
    let mut cooldown: u32 = 0;
    let mut energi: i64 = 0;
    let mut is_enabled = true;
    let mut is_owner = false;
    let mut is_partner = false;
    let mut is_premium = false;
    let mut is_group = false;
    let mut is_private = true;
    let mut is_admin = false;
    let mut is_bot_admin = false;
    let mut skip_registration = false;

    for arg in args {
        let Meta::NameValue(nv) = arg else { continue };
        let key = nv
            .path
            .get_ident()
            .map(|i| i.to_string())
            .unwrap_or_default();
        match key.as_str() {
            "name" => name = lit_str(&nv.value).unwrap_or_default(),
            "category" => category = lit_str(&nv.value).unwrap_or_default(),
            "description" => description = lit_str(&nv.value).unwrap_or_default(),
            "usage" => usage = lit_str(&nv.value).unwrap_or_default(),
            "example" => example = lit_str(&nv.value).unwrap_or_default(),
            "alias" => alias = lit_str_array(&nv.value),
            "cooldown" => cooldown = lit_u32(&nv.value).unwrap_or(0),
            "energi" => energi = lit_i64(&nv.value).unwrap_or(0),
            "is_enabled" => is_enabled = lit_bool(&nv.value).unwrap_or(true),
            "is_owner" => is_owner = lit_bool(&nv.value).unwrap_or(false),
            "is_premium" => is_premium = lit_bool(&nv.value).unwrap_or(false),
            "is_partner" => is_partner = lit_bool(&nv.value).unwrap_or(false),
            "is_group" => is_group = lit_bool(&nv.value).unwrap_or(false),
            "is_private" => is_private = lit_bool(&nv.value).unwrap_or(true),
            "is_admin" => is_admin = lit_bool(&nv.value).unwrap_or(false),
            "is_bot_admin" => is_bot_admin = lit_bool(&nv.value).unwrap_or(false),
            "skip_registration" => skip_registration = lit_bool(&nv.value).unwrap_or(false),
            _ => {}
        }
    }

    let ident = &func.sig.ident;
    let entry_reg = quote::format_ident!("__{}_REG", ident.to_string().to_uppercase());

    let expanded = quote! {
        #func

        #[allow(non_upper_case_globals)]
        #[::linkme::distributed_slice(::oktz_plugin_api::registry::PLUGINS)]
        static #entry_reg: ::oktz_plugin_api::registry::PluginInit = || {
            ::std::boxed::Box::leak(::std::boxed::Box::new(
                ::oktz_plugin_api::registry::PluginEntry {
                    meta: ::oktz_plugin_api::meta::PluginMeta {
                        name: #name.to_string(),
                        alias: vec![#(#alias.to_string()),*],
                        category: #category.to_string(),
                        description: #description.to_string(),
                        usage: #usage.to_string(),
                        example: #example.to_string(),
                        cooldown: #cooldown,
                        energi: #energi,
                        is_enabled: #is_enabled,
                        is_owner: #is_owner,
                        is_partner: #is_partner,
                        is_premium: #is_premium,
                        is_group: #is_group,
                        is_private: #is_private,
                        is_admin: #is_admin,
                        is_bot_admin: #is_bot_admin,
                        skip_registration: #skip_registration,
                        // `customTrigger` hanya dipakai 1 plugin (eval.js) dan
                        // butuh trigger bebas-prefix — belum ada di router.
                        custom_trigger: ::std::option::Option::None,
                    },
                    handler: ::std::sync::Arc::new(#ident),
                },
            ))
        };
    };
    expanded.into()
}

/// Buang satu lapisan `Expr::Group`.
///
/// Kalau argumen `#[plugin(...)]` diteruskan lewat `macro_rules!`, token-nya
/// dibungkus `Group` (invisible-delimiter), bukan `Lit` langsung. Tanpa
/// unwrap di sini, semua helper `lit_*` diam-diam mengembalikan default:
/// plugin terdaftar dengan `name: ""`, dan gejalanya "tidak ketemu saat
/// lookup" — jauh dari penyebabnya.
fn peel(e: &Expr) -> &Expr {
    match e {
        Expr::Group(g) => peel(&g.expr),
        other => other,
    }
}

fn lit_str(e: &Expr) -> Option<String> {
    match peel(e) {
        Expr::Lit(ExprLit {
            lit: Lit::Str(s), ..
        }) => Some(s.value()),
        _ => None,
    }
}

fn lit_str_array(e: &Expr) -> Vec<String> {
    match peel(e) {
        Expr::Array(arr) => arr.elems.iter().filter_map(lit_str).collect(),
        _ => Vec::new(),
    }
}

fn lit_u32(e: &Expr) -> Option<u32> {
    match peel(e) {
        Expr::Lit(ExprLit {
            lit: Lit::Int(i), ..
        }) => i.base10_parse().ok(),
        _ => None,
    }
}

fn lit_i64(e: &Expr) -> Option<i64> {
    match peel(e) {
        Expr::Lit(ExprLit {
            lit: Lit::Int(i), ..
        }) => i.base10_parse().ok(),
        _ => None,
    }
}

fn lit_bool(e: &Expr) -> Option<bool> {
    match peel(e) {
        Expr::Lit(ExprLit {
            lit: Lit::Bool(b), ..
        }) => Some(b.value),
        _ => None,
    }
}
