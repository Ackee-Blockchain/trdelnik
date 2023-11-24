use crate::idl::Idl;
use quote::{format_ident, ToTokens};
use syn::{parse_quote, parse_str};

/// Generates `program_client`'s `lib.rs` from [Idl] created from Anchor programs.
/// Disable regenerating the `use` statements with a used imports `use_modules`
///
/// _Note_: See the crate's tests for output example.
pub fn generate_source_code(idl: &Idl, use_modules: &[syn::ItemUse]) -> String {
    let mut output = "// DO NOT EDIT - automatically generated file (except `use` statements inside the `*_instruction` module\n".to_owned();
    let code = idl
        .programs
        .iter()
        .map(|idl_program| {
            let program_name = idl_program.name.snake_case.replace('-', "_");
            let instruction_module_name = format_ident!("{}_instruction", program_name);
            let module_name: syn::Ident = parse_str(&program_name).unwrap();
            let pubkey_bytes: syn::ExprArray = parse_str(&idl_program.id).unwrap();

            let instructions = idl_program
                .instruction_account_pairs
                .iter()
                .fold(
                    Vec::new(),
                    |mut instructions, (idl_instruction, idl_account_group)| {
                        let instruction_fn_name: syn::Ident =
                            parse_str(&idl_instruction.name.snake_case).unwrap();
                        let instruction_struct_name: syn::Ident =
                            parse_str(&idl_instruction.name.upper_camel_case).unwrap();
                        let account_struct_name: syn::Ident =
                            parse_str(&idl_account_group.name.upper_camel_case).unwrap();
                        let instruction_name: syn::Ident =
                            parse_str(&(idl_instruction.name.snake_case.clone() + "_ix")).unwrap();

                        let parameters = idl_instruction
                            .parameters
                            .iter()
                            .map(|(name, ty)| {
                                let name = format_ident!("i_{name}");
                                let ty: syn::Type = parse_str(ty).unwrap();
                                let parameter: syn::FnArg = parse_quote!(#name: #ty);
                                parameter
                            })
                            .collect::<Vec<_>>();

                        let accounts = idl_account_group
                            .accounts
                            .iter()
                            .map(|(name, ty)| {
                                let name = format_ident!("a_{name}");
                                // do not use fully qualified type for Pubkey
                                let ty = parse_str(ty).unwrap();
                                let ty: syn::Type = match &ty {
                                    syn::Type::Path(tp) => {
                                        let last_type =
                                            &tp.path.segments.last().unwrap().ident.to_string();
                                        if last_type == "Pubkey" {
                                            let t: syn::Type = parse_str(last_type).unwrap();
                                            t
                                        } else {
                                            // we expect only Pubkey, but if it is something different, than return fully qualified type
                                            ty
                                        }
                                    }
                                    _ => ty,
                                };
                                let account: syn::FnArg = parse_quote!(#name: #ty);
                                account
                            })
                            .collect::<Vec<_>>();

                        let field_parameters = idl_instruction
                            .parameters
                            .iter()
                            .map(|(name, _)| {
                                let name: syn::Ident = parse_str(name).unwrap();
                                let value = format_ident!("i_{name}");
                                let parameter: syn::FieldValue = parse_quote!(#name: #value);
                                parameter
                            })
                            .collect::<Vec<_>>();

                        let field_accounts = idl_account_group
                            .accounts
                            .iter()
                            .map(|(name, _)| {
                                let name: syn::Ident = parse_str(name).unwrap();
                                let value = format_ident!("a_{name}");
                                let account: syn::FieldValue = parse_quote!(#name: #value);
                                account
                            })
                            .collect::<Vec<_>>();

                        let instruction: syn::ItemFn = parse_quote! {
                            pub async fn #instruction_fn_name(
                                client: &Client,
                                #(#parameters,)*
                                #(#accounts,)*
                                signers: impl IntoIterator<Item = Keypair> + Send + 'static,
                            ) -> Result<EncodedConfirmedTransactionWithStatusMeta, ClientError> {
                                client.send_instruction(
                                    PROGRAM_ID,
                                    #module_name::instruction::#instruction_struct_name {
                                        #(#field_parameters,)*
                                    },
                                    #module_name::accounts::#account_struct_name {
                                        #(#field_accounts,)*
                                    },
                                    signers,
                                ).await
                            }
                        };

                        let instruction_raw: syn::ItemFn = parse_quote! {
                            pub  fn #instruction_name(
                                #(#parameters,)*
                                #(#accounts,)*
                            ) -> Instruction {
                                Instruction{
                                    program_id: PROGRAM_ID,
                                    data: #module_name::instruction::#instruction_struct_name {
                                        #(#field_parameters,)*
                                    }.data(),
                                    accounts: #module_name::accounts::#account_struct_name {
                                        #(#field_accounts,)*
                                    }.to_account_metas(None),
                                }
                            }
                        };

                        instructions.push(instruction);
                        instructions.push(instruction_raw);
                        instructions
                    },
                )
                .into_iter();

            let program_module: syn::ItemMod = parse_quote! {
                pub mod #instruction_module_name {
                    #(#use_modules)*
                    pub static PROGRAM_ID: Pubkey = Pubkey::new_from_array(#pubkey_bytes);
                    #(#instructions)*
                }
            };
            program_module.into_token_stream().to_string()
        })
        .collect::<String>();
    output.push_str(&code);
    output
}
