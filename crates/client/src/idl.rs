//! The `idl` module contains structs and functions for Anchor program code parsing.
//!
//! [Idl] example:
//!
//! ```rust,ignore
//! Idl {
//!     programs: [
//!         IdlProgram {
//!             name: IdlName {
//!                 snake_case: "turnstile",
//!                 upper_camel_case: "Turnstile",
//!             },
//!             id: "[216u8 , 55u8 , 200u8 , 93u8 , 189u8 , 81u8 , 94u8 , 109u8 , 14u8 , 249u8 , 244u8 , 106u8 , 68u8 , 214u8 , 222u8 , 190u8 , 9u8 , 25u8 , 199u8 , 75u8 , 79u8 , 230u8 , 94u8 , 137u8 , 51u8 , 187u8 , 193u8 , 48u8 , 87u8 , 222u8 , 175u8 , 163u8]",
//!             instruction_account_pairs: [
//!                 (
//!                     IdlInstruction {
//!                         name: IdlName {
//!                             snake_case: "initialize",
//!                             upper_camel_case: "Initialize",
//!                         },
//!                         parameters: [],
//!                     },
//!                     IdlAccountGroup {
//!                         name: IdlName {
//!                             snake_case: "initialize",
//!                             upper_camel_case: "Initialize",
//!                         },
//!                         accounts: [
//!                             (
//!                                 "state",
//!                                 "anchor_lang :: solana_program :: pubkey :: Pubkey",
//!                             ),
//!                             (
//!                                 "user",
//!                                 "anchor_lang :: solana_program :: pubkey :: Pubkey",
//!                             ),
//!                             (
//!                                 "system_program",
//!                                 "anchor_lang :: solana_program :: pubkey :: Pubkey",
//!                             ),
//!                         ],
//!                     },
//!                 ),
//!                 (
//!                     IdlInstruction {
//!                         name: IdlName {
//!                             snake_case: "coin",
//!                             upper_camel_case: "Coin",
//!                         },
//!                         parameters: [
//!                             (
//!                                 "dummy_arg",
//!                                 "String",
//!                             ),
//!                         ],
//!                     },
//!                     IdlAccountGroup {
//!                         name: IdlName {
//!                             snake_case: "update_state",
//!                             upper_camel_case: "UpdateState",
//!                         },
//!                         accounts: [
//!                             (
//!                                 "state",
//!                                 "anchor_lang :: solana_program :: pubkey :: Pubkey",
//!                             ),
//!                         ],
//!                     },
//!                 ),
//!                 (
//!                     IdlInstruction {
//!                         name: IdlName {
//!                             snake_case: "push",
//!                             upper_camel_case: "Push",
//!                         },
//!                         parameters: [],
//!                     },
//!                     IdlAccountGroup {
//!                         name: IdlName {
//!                             snake_case: "update_state",
//!                             upper_camel_case: "UpdateState",
//!                         },
//!                         accounts: [
//!                             (
//!                                 "state",
//!                                 "anchor_lang :: solana_program :: pubkey :: Pubkey",
//!                             ),
//!                         ],
//!                     },
//!                 ),
//!             ],
//!         },
//!     ],
//! }
//! ```

use std::path::{Path, PathBuf};

use heck::{ToSnakeCase, ToUpperCamelCase};
use quote::ToTokens;
use thiserror::Error;
static ACCOUNT_MOD_PREFIX: &str = "__client_accounts_";

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0:?}")]
    RustParsingError(#[from] syn::Error),
    #[error("missing or invalid program item: '{0}'")]
    MissingOrInvalidProgramItems(&'static str),
}

#[derive(Debug)]
pub struct Idl {
    pub programs: Vec<IdlProgram>,
}

#[derive(Debug)]
pub struct IdlName {
    pub snake_case: String,
    pub upper_camel_case: String,
}

#[derive(Debug)]
pub struct IdlProgram {
    pub name: IdlName,
    pub path: PathBuf,
    pub id: String,
    pub instruction_account_pairs: Vec<(IdlInstruction, IdlAccountGroup)>,
    pub program_invocations: Vec<String>,
}

#[derive(Debug)]
pub struct IdlInstruction {
    pub name: IdlName,
    pub parameters: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct IdlAccountGroup {
    pub name: IdlName,
    pub accounts: Vec<(String, String)>,
}

pub async fn parse_to_idl_program(
    name: String,
    code: &str,
    path: &Path,
) -> Result<IdlProgram, Error> {
    let mut static_program_id = None::<syn::ItemStatic>;
    let mut mod_private = None::<syn::ItemMod>;
    let mut mod_instruction = None::<syn::ItemMod>;
    let mut account_mods = Vec::<syn::ItemMod>::new();
    let mut context_structs = Vec::<syn::ItemStruct>::new();

    for item in syn::parse_file(code)?.items.into_iter() {
        match item {
            syn::Item::Struct(item_struct) => {
                if let Some(_lt) = &item_struct.generics.lt_token {
                    if let syn::GenericParam::Lifetime(_lifetime) =
                        item_struct.generics.params.first().unwrap()
                    {
                        context_structs.push(item_struct);
                    }
                }
            }
            syn::Item::Static(item_static) if item_static.ident == "ID" => {
                static_program_id = Some(item_static);
            }
            syn::Item::Mod(item_mod) => match item_mod.ident.to_string().as_str() {
                "__private" => mod_private = Some(item_mod),
                "instruction" => mod_instruction = Some(item_mod),
                _ => set_account_modules(&mut account_mods, item_mod),
            },
            _ => (),
        }
    }

    let static_program_id =
        static_program_id.ok_or(Error::MissingOrInvalidProgramItems("missing static ID"))?;
    let mod_private =
        mod_private.ok_or(Error::MissingOrInvalidProgramItems("missing mod private"))?;
    let mod_instruction = mod_instruction.ok_or(Error::MissingOrInvalidProgramItems(
        "missing mod instruction",
    ))?;

    let mut program_names = Vec::<String>::new();

    // ------ get Programs from Contex ------
    context_structs.iter().for_each(|context_struct| {
        if let syn::Fields::Named(named_fields) = &context_struct.fields {
            for x in named_fields.named.iter() {
                if let syn::Type::Path(type_path) = &x.ty {
                    for y in type_path.path.segments.iter() {
                        if y.ident == "Program" {
                            if let syn::PathArguments::AngleBracketed(
                                angel_bracketed_generic_args,
                            ) = &y.arguments
                            {
                                for z in angel_bracketed_generic_args.args.iter() {
                                    if let syn::GenericArgument::Type(syn::Type::Path(path)) = z {
                                        program_names.push(path.path.segments[0].ident.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });
    // ------ get program id ------

    // input example:
    // ```
    // pub static ID: anchor_lang::solana_program::pubkey::Pubkey =
    //     anchor_lang::solana_program::pubkey::Pubkey::new_from_array([216u8, 55u8,
    //                                                                  200u8, 93u8,
    //                                                                  189u8, 81u8,
    //                                                                  94u8, 109u8,
    //                                                                  14u8, 249u8,
    //                                                                  244u8, 106u8,
    //                                                                  68u8, 214u8,
    //                                                                  222u8, 190u8,
    //                                                                  9u8, 25u8,
    //                                                                  199u8, 75u8,
    //                                                                  79u8, 230u8,
    //                                                                  94u8, 137u8,
    //                                                                  51u8, 187u8,
    //                                                                  193u8, 48u8,
    //                                                                  87u8, 222u8,
    //                                                                  175u8,
    //                                                                  163u8]);
    // ```

    let program_id_bytes = {
        let new_pubkey_call = match *static_program_id.expr {
            syn::Expr::Call(new_pubkey_call) => new_pubkey_call,
            _ => {
                return Err(Error::MissingOrInvalidProgramItems(
                    "static ID: new pubkey call not found",
                ))
            }
        };
        match new_pubkey_call.args.into_iter().next() {
            Some(syn::Expr::Array(pubkey_bytes)) => pubkey_bytes,
            _ => {
                return Err(Error::MissingOrInvalidProgramItems(
                    "static ID: pubkey bytes not found",
                ))
            }
        }
    };

    // ------ get instruction_item_fns ------

    // input example:
    // ```
    // mod __private {
    //     pub mod __global {
    //         use super::*;
    //         #[inline(never)]
    //         pub fn initialize(program_id: &Pubkey, accounts: &[AccountInfo],
    //                           ix_data: &[u8]) -> ProgramResult {
    //             let ix =
    //                 instruction::Initialize::deserialize(&mut &ix_data[..]).map_err(|_|
    //                                                                                     anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
    //             let instruction::Initialize = ix;
    //             let mut remaining_accounts: &[AccountInfo] = accounts;
    //             let mut accounts =
    //                 Initialize::try_accounts(program_id, &mut remaining_accounts,
    //                                          ix_data)?;
    //             turnstile::initialize(Context::new(program_id, &mut accounts,
    //                                                remaining_accounts))?;
    //             accounts.exit(program_id)
    //         }
    // ```

    let instruction_item_fns = {
        let items = mod_private
            .content
            .map(|(_, items)| items)
            .unwrap_or_default();
        let item_mod_global = items
            .into_iter()
            .find_map(|item| match item {
                syn::Item::Mod(item_mod) if item_mod.ident == "__global" => Some(item_mod),
                _ => None?,
            })
            .ok_or(Error::MissingOrInvalidProgramItems(
                "mod private: mod global not found",
            ))?;
        let items = item_mod_global
            .content
            .map(|(_, items)| items)
            .unwrap_or_default();
        items.into_iter().filter_map(|item| match item {
            syn::Item::Fn(item_fn) => Some(item_fn),
            _ => None,
        })
    };

    // ------ get instruction + account group names ------

    // input example:
    // ```
    //         pub fn initialize(program_id: &Pubkey, accounts: &[AccountInfo],
    //                           ix_data: &[u8]) -> ProgramResult {
    //             let ix =
    //                 instruction::Initialize::deserialize(&mut &ix_data[..]).map_err(|_|
    //                                                                                     anchor_lang::__private::ErrorCode::InstructionDidNotDeserialize)?;
    //             let instruction::Initialize = ix;
    //             let mut remaining_accounts: &[AccountInfo] = accounts;
    //             let mut accounts =
    //                 Initialize::try_accounts(program_id, &mut remaining_accounts,
    //                                          ix_data)?;
    //             turnstile::initialize(Context::new(program_id, &mut accounts,
    //                                                remaining_accounts))?;
    //             accounts.exit(program_id)
    //         }
    // ```

    let mut instruction_account_pairs = Vec::new();
    instruction_item_fns
        .into_iter()
        .map(|item_fn| {
            // stmt example: `let mut accounts = UpdateState::try_accounts(program_id, &mut remaining_accounts, ix_data)?;`
            let account_group_name = item_fn.block.stmts.into_iter().find_map(|stmt| {
            let local = if let syn::Stmt::Local(local) = stmt {
                local
            } else {
                None?
            };
            if !matches!(&local.pat, syn::Pat::Ident(pat_ident) if pat_ident.ident == "__accounts") {
                None?
            }
            let init_expr = *local.init?.1;
            let expr_try_expr = match init_expr {
                syn::Expr::Try(expr_try) => *expr_try.expr,
                _ => None?
            };
            let expr_call_func = match expr_try_expr {
                syn::Expr::Call(expr_call) => *expr_call.func,
                _ => None?
            };
            let account_group_name = match expr_call_func {
                syn::Expr::Path(expr_path) => expr_path.path.segments.into_iter().next()?.ident,
                _ => None?
            };
            Some(account_group_name.to_string())
        })?;

            let instruction_name = item_fn.sig.ident.to_string();
            let idl_instruction = IdlInstruction {
                name: IdlName {
                    upper_camel_case: instruction_name.to_upper_camel_case(),
                    snake_case: instruction_name,
                },
                parameters: Vec::new(),
            };
            let idl_account = IdlAccountGroup {
                name: IdlName {
                    snake_case: account_group_name.to_snake_case(),
                    upper_camel_case: account_group_name,
                },
                accounts: Vec::new(),
            };
            Some((idl_instruction, idl_account))
        })
        .try_for_each(|pair| {
            if let Some(pair) = pair {
                instruction_account_pairs.push(pair);
                Ok(())
            } else {
                Err(Error::MissingOrInvalidProgramItems(
                    "statement with `accounts` not found",
                ))
            }
        })?;

    // ------ get instruction parameters ------

    // input example:
    // ```
    // pub mod instruction {
    //     use super::*;
    //     pub mod state {
    //         use super::*;
    //     }
    // // **
    //     pub struct Initialize;
    // // **
    //     pub struct Coin {
    //         pub dummy_arg: String,
    //     }
    // ```

    let mut instruction_mod_items = mod_instruction
        .content
        .ok_or(Error::MissingOrInvalidProgramItems(
            "instruction mod: empty content",
        ))?
        .1
        .into_iter();

    for (idl_instruction, _) in &mut instruction_account_pairs {
        let instruction_struct_name = &idl_instruction.name.upper_camel_case;

        let instruction_item_struct_fields = instruction_mod_items
            .find_map(|item| {
                let instruction_item_struct = match item {
                    syn::Item::Struct(item_struct)
                        if item_struct.ident == instruction_struct_name =>
                    {
                        item_struct
                    }
                    _ => None?,
                };
                let fields = match instruction_item_struct.fields {
                    syn::Fields::Named(fields_named) => fields_named.named,
                    syn::Fields::Unit => syn::punctuated::Punctuated::new(),
                    syn::Fields::Unnamed(_) => None?,
                };
                Some(fields.into_iter())
            })
            .ok_or(Error::MissingOrInvalidProgramItems("instruction struct"))?;

        idl_instruction.parameters = instruction_item_struct_fields
            .map(|field| {
                let parameter_name = field.ident.unwrap().to_string();
                let parameter_id_type = field.ty.into_token_stream().to_string();
                (parameter_name, parameter_id_type)
            })
            .collect();
    }

    // ------ get accounts ------

    // input example:
    // ```
    // pub(crate) mod __client_accounts_initialize {
    //     use super::*;
    //     use anchor_lang::prelude::borsh;
    //     pub struct Initialize {
    //         pub state: anchor_lang::solana_program::pubkey::Pubkey,
    //         pub user: anchor_lang::solana_program::pubkey::Pubkey,
    //         pub system_program: anchor_lang::solana_program::pubkey::Pubkey,
    //     }
    // ```

    for account_mod_item in account_mods {
        let account_struct_name = account_mod_item
            .ident
            .to_string()
            .strip_prefix(ACCOUNT_MOD_PREFIX)
            .unwrap()
            .to_upper_camel_case();

        let account_item_struct = account_mod_item
            .content
            .ok_or(Error::MissingOrInvalidProgramItems(
                "account mod: empty content",
            ))?
            .1
            .into_iter()
            .find_map(|item| match item {
                syn::Item::Struct(item_struct) if item_struct.ident == account_struct_name => {
                    Some(item_struct)
                }
                _ => None?,
            })
            .ok_or(Error::MissingOrInvalidProgramItems(
                "account mod: struct not found",
            ))?;

        let account_item_struct_fields = match account_item_struct.fields {
            syn::Fields::Named(fields_named) => fields_named.named,
            syn::Fields::Unit => syn::punctuated::Punctuated::new(),
            syn::Fields::Unnamed(_) => {
                return Err(Error::MissingOrInvalidProgramItems(
                    "account struct: unnamed fields not allowed",
                ))
            }
        };

        let accounts = account_item_struct_fields
            .into_iter()
            .map(|field| {
                let account_name = field.ident.unwrap().to_string();
                let account_id_type = field.ty.into_token_stream().to_string();
                (account_name, account_id_type)
            })
            .collect::<Vec<_>>();

        for (_, idl_account_group) in &mut instruction_account_pairs {
            if idl_account_group.name.upper_camel_case == account_struct_name {
                idl_account_group.accounts = accounts.clone();
            }
        }
    }

    // ------ // ------

    Ok(IdlProgram {
        name: IdlName {
            upper_camel_case: name.to_upper_camel_case(),
            snake_case: name,
        },
        path: path.to_path_buf(),
        id: program_id_bytes.into_token_stream().to_string(),
        instruction_account_pairs,
        program_invocations: program_names,
    })
}

fn set_account_modules(account_modules: &mut Vec<syn::ItemMod>, item_module: syn::ItemMod) {
    if item_module
        .ident
        .to_string()
        .starts_with(ACCOUNT_MOD_PREFIX)
    {
        account_modules.push(item_module);
        return;
    }
    let modules = item_module
        .content
        .ok_or(Error::MissingOrInvalidProgramItems(
            "account mod: empty content",
        ))
        .unwrap()
        .1;
    for module in modules {
        if let syn::Item::Mod(nested_module) = module {
            set_account_modules(account_modules, nested_module);
        }
    }
}
