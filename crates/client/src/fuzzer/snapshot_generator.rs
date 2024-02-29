// To generate the snapshot data types, we need to first find all context struct within the program and parse theirs accounts.
// The parsing of individual Anchor accounts is done using Anchor syn parser:
// https://github.com/coral-xyz/anchor/blob/master/lang/syn/src/parser/accounts/mod.rs

use std::collections::HashMap;
use std::{error::Error, fs::File, io::Read};

use anchor_lang::anchor_syn::{AccountField, Ty};
use cargo_metadata::camino::Utf8PathBuf;
use heck::ToUpperCamelCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::parse::{Error as ParseError, Result as ParseResult};
use syn::spanned::Spanned;
use syn::{parse_quote, Attribute, Fields, GenericArgument, Item, ItemStruct, PathArguments, Type};

use anchor_lang::anchor_syn::parser::accounts::parse_account_field;

pub fn generate_snapshots_code(code_path: &[(String, Utf8PathBuf)]) -> Result<String, String> {
    let code = code_path.iter().map(|(code, path)| {
        let mut mod_program = None::<syn::ItemMod>;
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| e.to_string())?;

        let parse_result = syn::parse_file(&content).map_err(|e| e.to_string())?;

        // locate the program module to extract instructions and corresponding Context structs.
        for item in parse_result.items.iter() {
            if let Item::Mod(module) = item {
                // Check if the module has the #[program] attribute
                if has_program_attribute(&module.attrs) {
                    mod_program = Some(module.clone())
                }
            }
        }

        let mod_program = mod_program.ok_or("module with program attribute not found")?;

        let (_, items) = mod_program
            .content
            .ok_or("the content of program module is missing")?;

        let ix_ctx_pairs = get_ix_ctx_pairs(&items)?;

        let (structs, impls, type_aliases) = get_snapshot_structs_and_impls(code, &ix_ctx_pairs)?;

        let use_statements = quote! {
            use trdelnik_client::anchor_lang::{prelude::*, self};
            use trdelnik_client::fuzzing::FuzzingError;
        }
        .into_token_stream();
        Ok(format!(
            "{}{}{}{}",
            use_statements, structs, impls, type_aliases
        ))
    });

    code.into_iter().collect()
}

/// Creates new snapshot structs with fields wrapped in Option<_> if approriate and the
/// respective implementations with snapshot deserialization methods
fn get_snapshot_structs_and_impls(
    code: &str,
    ix_ctx_pairs: &[(Ident, GenericArgument)],
) -> Result<(String, String, String), String> {
    let mut structs = String::new();
    let mut impls = String::new();
    let mut type_aliases = String::new();
    let parse_result = syn::parse_file(code).map_err(|e| e.to_string())?;
    let mut unique_ctxs: HashMap<GenericArgument, Ident> = HashMap::new();
    for (ix, ctx) in ix_ctx_pairs {
        let mut ctx_ident = None;
        let ix_name = ix.to_string().to_upper_camel_case();
        if let GenericArgument::Type(syn::Type::Path(tp)) = ctx {
            ctx_ident = tp.path.get_ident().cloned();
        }
        let ctx_ident =
            ctx_ident.ok_or(format!("malformed parameters of {} instruction", ix_name))?;

        // If ctx is in the HashMap, we do not need to generate deserialization code again, we can only create a type alias
        match unique_ctxs.get(ctx) {
            Some(base_ix_snapshot_name) => {
                let snapshot_alias_name = format_ident!("{}Snapshot", ix_name);
                let type_alias =
                    quote! {pub type #snapshot_alias_name<'info> = #base_ix_snapshot_name<'info>;};
                type_aliases = format!("{}{}", type_aliases, type_alias.into_token_stream());
            }
            None => {
                // recursively find the context struct and create a new version with wrapped fields into Option
                if let Some(ctx_struct_item) = find_ctx_struct(&parse_result.items, &ctx_ident) {
                    let fields_parsed = if let Fields::Named(f) = ctx_struct_item.fields.clone() {
                        let field_deser: ParseResult<Vec<AccountField>> =
                            f.named.iter().map(parse_account_field).collect();
                        field_deser
                    } else {
                        Err(ParseError::new(
                            ctx_struct_item.fields.span(),
                            "Context struct parse errror.",
                        ))
                    }
                    .map_err(|e| e.to_string())?;

                    let ix_snapshot_name = format_ident!("{}Snapshot", ix_name);
                    let wrapped_struct =
                        create_snapshot_struct(&ix_snapshot_name, ctx_struct_item, &fields_parsed)
                            .unwrap();
                    let deser_code =
                        deserialize_ctx_struct_anchor(&ix_snapshot_name, &fields_parsed)
                            .map_err(|e| e.to_string())?;
                    structs = format!("{}{}", structs, wrapped_struct.into_token_stream());
                    impls = format!("{}{}", impls, deser_code.into_token_stream());
                    unique_ctxs.insert(ctx.clone(), ix_snapshot_name);
                } else {
                    return Err(format!("The Context struct {} was not found", ctx_ident));
                }
            }
        };
    }

    Ok((structs, impls, type_aliases))
}

/// Iterates through items and finds functions with the Context<_> parameter. Returns pairs with the function name and the Context's inner type.
fn get_ix_ctx_pairs(items: &[Item]) -> Result<Vec<(Ident, GenericArgument)>, String> {
    let mut ix_ctx_pairs = Vec::new();
    for item in items {
        if let syn::Item::Fn(func) = item {
            let func_name = &func.sig.ident;
            let first_param_type = if let Some(param) = func.sig.inputs.iter().next() {
                let mut ty = None::<GenericArgument>;
                if let syn::FnArg::Typed(t) = param {
                    if let syn::Type::Path(tp) = *t.ty.clone() {
                        if let Some(seg) = tp.path.segments.iter().next() {
                            if let PathArguments::AngleBracketed(arg) = &seg.arguments {
                                ty = arg.args.first().cloned();
                            }
                        }
                    }
                }
                ty
            } else {
                None
            };

            let first_param_type = first_param_type.ok_or(format!(
                "The function {} does not have the Context parameter and is malformed.",
                func_name
            ))?;

            ix_ctx_pairs.push((func_name.clone(), first_param_type));
        }
    }
    Ok(ix_ctx_pairs)
}

/// Recursively find a struct with a given `name`
fn find_ctx_struct<'a>(items: &'a Vec<syn::Item>, name: &'a syn::Ident) -> Option<&'a ItemStruct> {
    for item in items {
        if let Item::Struct(struct_item) = item {
            if struct_item.ident == *name {
                return Some(struct_item);
            }
        }
    }

    // if the ctx struct is not found on the first level, recursively continue to search in submodules
    for item in items {
        if let Item::Mod(mod_item) = item {
            if let Some((_, items)) = &mod_item.content {
                let r = find_ctx_struct(items, name);
                if r.is_some() {
                    return r;
                }
            };
        }
    }

    None
}

fn is_boxed(ty: &anchor_lang::anchor_syn::Ty) -> bool {
    match ty {
        Ty::Account(acc) => acc.boxed,
        Ty::InterfaceAccount(acc) => acc.boxed,
        _ => false,
    }
}

/// Determines if an Account should be wrapped into the `Option` type.
/// The function returns true if the account has the init or close constraints set
/// or if it is wrapped into the `Option` type.
fn is_optional(parsed_field: &AccountField) -> bool {
    let is_optional = match parsed_field {
        AccountField::Field(field) => field.is_optional,
        AccountField::CompositeField(_) => false,
    };
    let constraints = match parsed_field {
        AccountField::Field(f) => &f.constraints,
        AccountField::CompositeField(f) => &f.constraints,
    };

    constraints.init.is_some() || constraints.is_close() || is_optional
}

/// Creates new Snapshot struct from the context struct. Removes Box<> types.
fn create_snapshot_struct(
    snapshot_name: &Ident,
    orig_struct: &ItemStruct,
    parsed_fields: &[AccountField],
) -> Result<TokenStream, Box<dyn Error>> {
    let wrapped_fields = match orig_struct.fields.clone() {
        Fields::Named(named) => {
            let field_wrappers =
                named
                    .named
                    .iter()
                    .zip(parsed_fields)
                    .map(|(field, parsed_field)| {
                        let field_name = &field.ident;
                        let mut field_type = &field.ty;
                        #[allow(unused_assignments)]
                        let mut is_account_info = false;
                        if let AccountField::Field(f) = parsed_field {
                            if f.is_optional {
                                // remove option
                                if let Some(ty) = extract_inner_type(field_type) {
                                    field_type = ty;
                                }
                            }
                            if is_boxed(&f.ty) {
                                // remove box
                                if let Some(ty) = extract_inner_type(field_type) {
                                    field_type = ty;
                                }
                            }
                            is_account_info = field_type
                                .to_token_stream()
                                .to_string()
                                .replace(' ', "")
                                .starts_with("AccountInfo<");
                        }
                        else {
                            println!("\x1b[1;93mWarning\x1b[0m: The context `{}` has a field named `{}` of composite type `{}`. \
                                The automatic deserialization of composite types is currently not supported. You will have \
                                to implement it manually in the generated `accounts_snapshots.rs` file. The field deserialization \
                                was replaced by a `todo!()` macro. Also, you might want to adapt the corresponding FuzzInstruction \
                                variants in `fuzz_instructions.rs` file.",
                                orig_struct.ident, field_name.to_token_stream(), field_type.to_token_stream()
                            );
                        }

                        match (is_optional(parsed_field), is_account_info) {
                            (true, true) => {
                                Ok(quote! {pub #field_name: Option<&'info #field_type>,})
                            }
                            (true, _) => Ok(quote! {pub #field_name: Option<#field_type>,}),
                            (_, true) => Ok(quote! {pub #field_name: &'info #field_type,}),
                            _ => Ok(quote! {pub #field_name: #field_type,}),
                        }
                    });

            let field_wrappers: Result<Vec<_>, Box<dyn Error>> =
                field_wrappers.into_iter().collect();
            let field_wrappers = field_wrappers?;
            quote! {
                { #(#field_wrappers)* }
            }
        }

        _ => return Err("Only structs with named fields are supported".into()),
    };

    // Generate the new struct with Option-wrapped fields
    let generated_struct: syn::ItemStruct = parse_quote! {
        pub struct #snapshot_name<'info> #wrapped_fields
    };

    Ok(generated_struct.to_token_stream())
}

fn extract_inner_type(field_type: &Type) -> Option<&Type> {
    if let syn::Type::Path(type_path) = field_type {
        let segment = type_path.path.segments.last()?;
        let ident = &segment.ident;

        if ident == "Box" || ident == "Option" {
            if let PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                    return Some(inner_type);
                }
            }
        }
    }

    None
}

/// Generates code to deserialize the snapshot structs.
fn deserialize_ctx_struct_anchor(
    snapshot_name: &Ident,
    parsed_fields: &[AccountField],
) -> Result<TokenStream, Box<dyn Error>> {
    let names_deser_pairs: Vec<(TokenStream, TokenStream)> = parsed_fields
        .iter()
        .map(|parsed_f| match parsed_f {
            AccountField::Field(f) => {
                let field_name = f.ident.clone();
                let is_optional = is_optional(parsed_f);
                let deser_tokens = match ty_to_tokens(&f.ty) {
                    Some((return_type, deser_method)) => deserialize_account_tokens(
                        &field_name,
                        is_optional,
                        return_type,
                        deser_method,
                    ),
                    None if matches!(&f.ty, Ty::UncheckedAccount) => {
                        acc_unchecked_tokens(&field_name, is_optional)
                    }
                    None => acc_info_tokens(&field_name, is_optional),
                };
                (
                    quote! {#field_name},
                    quote! {
                        #deser_tokens
                    },
                )
            }
            AccountField::CompositeField(f) => {
                let field_name = f.ident.clone();
                (
                    quote! { #field_name },
                    quote! { let #field_name = todo!(); },
                )
            }
        })
        .collect();

    let (names, fields_deser): (Vec<_>, Vec<_>) = names_deser_pairs.iter().cloned().unzip();

    let generated_deser_impl: syn::Item = parse_quote! {
        impl<'info> #snapshot_name<'info> {
            pub fn deserialize_option(
                accounts: &'info mut [Option<AccountInfo<'info>>],
            ) -> core::result::Result<Self, FuzzingError> {
                let mut accounts_iter = accounts.iter();

                #(#fields_deser)*

                Ok(Self {
                    #(#names),*
                })
            }
        }
    };

    Ok(generated_deser_impl.to_token_stream())
}

/// Get the identifier (name) of the passed sysvar type.
fn sysvar_to_ident(sysvar: &anchor_lang::anchor_syn::SysvarTy) -> String {
    let str = match sysvar {
        anchor_lang::anchor_syn::SysvarTy::Clock => "Clock",
        anchor_lang::anchor_syn::SysvarTy::Rent => "Rent",
        anchor_lang::anchor_syn::SysvarTy::EpochSchedule => "EpochSchedule",
        anchor_lang::anchor_syn::SysvarTy::Fees => "Fees",
        anchor_lang::anchor_syn::SysvarTy::RecentBlockhashes => "RecentBlockhashes",
        anchor_lang::anchor_syn::SysvarTy::SlotHashes => "SlotHashes",
        anchor_lang::anchor_syn::SysvarTy::SlotHistory => "SlotHistory",
        anchor_lang::anchor_syn::SysvarTy::StakeHistory => "StakeHistory",
        anchor_lang::anchor_syn::SysvarTy::Instructions => "Instructions",
        anchor_lang::anchor_syn::SysvarTy::Rewards => "Rewards",
    };
    str.into()
}

/// Converts passed account type to token streams. The function returns a pair of streams where the first
/// variable in the pair is the type itself and the second is a fully qualified function to deserialize
/// the given type.
pub fn ty_to_tokens(ty: &anchor_lang::anchor_syn::Ty) -> Option<(TokenStream, TokenStream)> {
    let (return_type, deser_method) = match ty {
        Ty::AccountInfo | Ty::UncheckedAccount => return None,
        Ty::SystemAccount => (
            quote! { SystemAccount<'_>},
            quote!(anchor_lang::accounts::system_account::SystemAccount::try_from),
        ),
        Ty::Sysvar(sysvar) => {
            let id = syn::Ident::new(&sysvar_to_ident(sysvar), Span::call_site());
            (
                quote! { Sysvar<#id>},
                quote!(anchor_lang::accounts::sysvar::Sysvar::from_account_info),
            )
        }
        Ty::Signer => (
            quote! { Signer<'_>},
            quote!(anchor_lang::accounts::signer::Signer::try_from),
        ),
        Ty::Account(acc) => {
            let path = &acc.account_type_path;
            (
                quote! { anchor_lang::accounts::account::Account<#path>},
                quote! {anchor_lang::accounts::account::Account::try_from},
            )
        }
        Ty::AccountLoader(acc) => {
            let path = &acc.account_type_path;
            (
                quote! { anchor_lang::accounts::account_loader::AccountLoader<#path>},
                quote! {anchor_lang::accounts::account_loader::AccountLoader::try_from},
            )
        }
        Ty::Program(prog) => {
            let path = &prog.account_type_path;
            (
                quote! { anchor_lang::accounts::program::Program<#path>},
                quote!(anchor_lang::accounts::program::Program::try_from),
            )
        }
        Ty::Interface(interf) => {
            let path = &interf.account_type_path;
            (
                quote! { anchor_lang::accounts::interface::Interface<#path>},
                quote! {anchor_lang::accounts::interface::Interface::try_from},
            )
        }
        Ty::InterfaceAccount(interf_acc) => {
            let path = &interf_acc.account_type_path;
            (
                quote! { anchor_lang::accounts::interface_account::InterfaceAccount<#path>},
                quote! {anchor_lang::accounts::interface_account::InterfaceAccount::try_from},
            )
        }
        Ty::ProgramData => return None,
    };
    Some((return_type, deser_method))
}

/// Generates the code necessary to deserialize an account
fn deserialize_account_tokens(
    name: &syn::Ident,
    is_optional: bool,
    return_type: TokenStream,
    deser_method: TokenStream,
) -> TokenStream {
    if is_optional {
        let name_str = name.to_string();
        // TODO make this more idiomatic
        quote! {
            let #name:Option<#return_type> = accounts_iter
            .next()
            .ok_or(FuzzingError::NotEnoughAccounts(#name_str.to_string()))?
            .as_ref()
            .map(|acc| {
                if acc.key() != PROGRAM_ID {
                    #deser_method(acc).map_err(|_| FuzzingError::CannotDeserializeAccount(#name_str.to_string()))
                } else {Err(FuzzingError::OptionalAccountNotProvided(
                        #name_str.to_string(),
                    ))
                }
            })
            .transpose()
            .unwrap_or(None);
        }
    } else {
        let name_str = name.to_string();
        quote! {
            let #name: #return_type = accounts_iter
            .next()
            .ok_or(FuzzingError::NotEnoughAccounts(#name_str.to_string()))?
            .as_ref()
            .map(#deser_method)
            .ok_or(FuzzingError::AccountNotFound(#name_str.to_string()))?
            // TODO It would be helpful to do something like line below.
            // where we propagate anchor error
            // However I suggest that this is not possible right now as for
            // fuzz_example3 the anchor_lang has version 0.28.0. However trdelnik
            // uses 0.29.0 I think this is the reason why the '?' operator cannot propagate
            // the error even though I implemnted From<anchor_lang::error::Error> trait
            // that i
            // .map_err(|e| e.with_account_name(#name_str).into())?;
            .map_err(|_| FuzzingError::CannotDeserializeAccount(#name_str.to_string()))?;
        }
    }
}

/// Generates the code used with raw accounts as AccountInfo
fn acc_info_tokens(name: &syn::Ident, is_optional: bool) -> TokenStream {
    let name_str = name.to_string();
    if is_optional {
        quote! {
            let #name = accounts_iter
            .next()
            .ok_or(FuzzingError::NotEnoughAccounts(#name_str.to_string()))?
            .as_ref();
        }
    } else {
        quote! {
            let #name = accounts_iter
            .next()
            .ok_or(FuzzingError::NotEnoughAccounts(#name_str.to_string()))?
            .as_ref()
            .ok_or(FuzzingError::AccountNotFound(#name_str.to_string()))?;
        }
    }
}

/// Generates the code used with Unchecked accounts
fn acc_unchecked_tokens(name: &syn::Ident, is_optional: bool) -> TokenStream {
    let name_str = name.to_string();
    if is_optional {
        quote! {
            let #name = accounts_iter
            .next()
            .ok_or(FuzzingError::NotEnoughAccounts(#name_str.to_string()))?
            .as_ref()
            .map(anchor_lang::accounts::unchecked_account::UncheckedAccount::try_from);
        }
    } else {
        quote! {
            let #name = accounts_iter
            .next()
            .ok_or(FuzzingError::NotEnoughAccounts(#name_str.to_string()))?
            .as_ref()
            .map(anchor_lang::accounts::unchecked_account::UncheckedAccount::try_from)
            .ok_or(FuzzingError::AccountNotFound(#name_str.to_string()))?;
        }
    }
}

/// Checks if the program attribute is present
fn has_program_attribute(attrs: &Vec<Attribute>) -> bool {
    for attr in attrs {
        if attr.path.is_ident("program") {
            return true;
        }
    }
    false
}
