use std::collections::HashMap;

use crate::idl::Idl;
use quote::{format_ident, ToTokens};
use syn::{parse_quote, parse_str};

/// Generates `fuzz_instructions.rs` from [Idl] created from Anchor programs.
pub fn generate_source_code(idl: &Idl) -> String {
    let code = idl
        .programs
        .iter()
        .map(|idl_program| {
            let program_name = idl_program.name.snake_case.replace('-', "_");
            let fuzz_instructions_module_name = format_ident!("{}_fuzz_instructions", program_name);
            let module_name: syn::Ident = parse_str(&program_name).unwrap();
            let mut accounts_from_instr_input: Vec<(String, String)> = vec![];

            let instructions = idl_program
                .instruction_account_pairs
                .iter()
                .fold(
                    Vec::new(),
                    |mut instructions, (idl_instruction, _idl_account_group)| {
                        let instruction_struct_name: syn::Ident =
                            parse_str(&idl_instruction.name.upper_camel_case).unwrap();

                        let instruction: syn::Variant = parse_quote! {
                            #instruction_struct_name(#instruction_struct_name)
                        };

                        instructions.push(instruction);
                        instructions
                    },
                )
                .into_iter();

            let instructions_data = idl_program
                .instruction_account_pairs
                .iter()
                .fold(
                    Vec::new(),
                    |mut instructions_data, (idl_instruction, idl_account_group)| {
                        let instruction_name: syn::Ident =
                            format_ident!("{}", &idl_instruction.name.upper_camel_case);

                        let instruction_data_name: syn::Ident =
                            format_ident!("{}Data", &idl_instruction.name.upper_camel_case);

                        let instruction_accounts_name: syn::Ident =
                            format_ident!("{}Accounts", &idl_instruction.name.upper_camel_case);

                        let parameters = idl_instruction
                            .parameters
                            .iter()
                            .map(|(name, ty)| {
                                let name_ident = format_ident!("{name}");
                                // TODO What about custom Enums and Structs on Instr Input ?
                                let ty = parse_str(ty).unwrap();
                                let ty: syn::Type = match &ty {
                                    syn::Type::Path(tp) => {
                                        let last_type =
                                            &tp.path.segments.last().unwrap().ident.to_string();
                                        if last_type == "Pubkey" {
                                            let t: syn::Type = parse_str("AccountId").unwrap();
                                            accounts_from_instr_input
                                                .push((name.to_string(), "AccountId".to_string()));
                                            t
                                        } else {
                                            ty
                                        }
                                    }
                                    _ => ty,
                                };
                                let parameter: syn::FnArg = parse_quote!(#name_ident: #ty);
                                parameter
                            })
                            .collect::<Vec<_>>();

                        let accounts = idl_account_group
                            .accounts
                            .iter()
                            .map(|(name, _ty)| {
                                let name = format_ident!("{name}");
                                let account: syn::FnArg = parse_quote!(#name: AccountId);
                                account
                            })
                            .collect::<Vec<_>>();

                        let ix_enum_variant: syn::ItemStruct = parse_quote! {
                            #[derive(Arbitrary, Clone)]
                            pub struct #instruction_name {
                                 pub accounts: #instruction_accounts_name,
                                 pub data: #instruction_data_name
                            }

                        };

                        let ix_accounts: syn::ItemStruct = parse_quote! {
                            #[derive(Arbitrary, Clone)]
                            pub struct #instruction_accounts_name {
                                 #(pub #accounts),*
                            }

                        };
                        let ix_data: syn::ItemStruct = parse_quote! {
                            #[derive(Arbitrary, Clone)]
                            pub struct #instruction_data_name {
                                 #(pub #parameters),*
                            }

                        };

                        instructions_data.push(ix_enum_variant);
                        instructions_data.push(ix_accounts);
                        instructions_data.push(ix_data);
                        instructions_data
                    },
                )
                .into_iter();

            let instructions_ixops_impls = idl_program
                .instruction_account_pairs
                .iter()
                .fold(
                    Vec::new(),
                    |mut instructions_ixops_impl, (idl_instruction, idl_account_group)| {
                        let instruction_name: syn::Ident =
                            format_ident!("{}", &idl_instruction.name.upper_camel_case);

                        let ix_snapshot: syn::Ident =
                            format_ident!("{}Snapshot", &idl_instruction.name.upper_camel_case);

                        let parameters = idl_instruction
                            .parameters
                            .iter()
                            .map(|(name, _ty)| {
                                let name = format_ident!("{name}");
                                let parameter: syn::FnArg = parse_quote!(#name: todo!());
                                parameter
                            })
                            .collect::<Vec<_>>();

                        let accounts = idl_account_group
                            .accounts
                            .iter()
                            .map(|(name, _ty)| {
                                let name = format_ident!("{name}");
                                let account: syn::FnArg = parse_quote!(#name: todo!());
                                account
                            })
                            .collect::<Vec<_>>();

                        let ix_impl: syn::ItemImpl = parse_quote! {
                            impl<'info> IxOps<'info> for #instruction_name {
                                type IxData = #module_name::instruction::#instruction_name;
                                type IxAccounts = FuzzAccounts;
                                type IxSnapshot = #ix_snapshot<'info>;

                                fn get_data(
                                    &self,
                                    _client: &mut impl FuzzClient,
                                    _fuzz_accounts: &mut FuzzAccounts,
                                ) -> Result<Self::IxData, FuzzingError> {
                                    let data = #module_name::instruction::#instruction_name {
                                        #(#parameters),*
                                    };
                                    Ok(data)
                                }

                                fn get_accounts(
                                &self,
                                client: &mut impl FuzzClient,
                                fuzz_accounts: &mut FuzzAccounts,
                                ) -> Result<(Vec<Keypair>, Vec<AccountMeta>), FuzzingError> {
                                    let signers = vec![todo!()];
                                    let acc_meta = #module_name::accounts::#instruction_name {
                                        #(#accounts),*
                                    }
                                    .to_account_metas(None);

                                    Ok((signers, acc_meta))
                                }
                            }

                        };

                        instructions_ixops_impl.push(ix_impl);
                        instructions_ixops_impl
                    },
                )
                .into_iter();

            let mut fuzz_accounts = idl_program.instruction_account_pairs.iter().fold(
                HashMap::new(),
                |mut fuzz_accounts, (_idl_instruction, idl_account_group)| {
                    idl_account_group.accounts.iter().fold(
                        &mut fuzz_accounts,
                        |fuzz_accounts, (name, _ty)| {
                            let name = format_ident!("{name}");
                            fuzz_accounts.entry(name).or_insert_with(|| "".to_string());
                            fuzz_accounts
                        },
                    );
                    fuzz_accounts
                },
            );

            fuzz_accounts.extend(accounts_from_instr_input.iter().fold(
                HashMap::new(),
                |mut fuzz_accounts, (name, _ty)| {
                    let name = format_ident!("{name}");
                    fuzz_accounts.entry(name).or_insert_with(|| "".to_string());
                    fuzz_accounts
                },
            ));

            // this ensures that the order of accounts is deterministic
            // so we can use expected generated template within tests
            let mut sorted_fuzz_accounts: Vec<_> = fuzz_accounts.keys().collect();
            sorted_fuzz_accounts.sort();

            let fuzzer_module: syn::ItemMod = parse_quote! {
                pub mod #fuzz_instructions_module_name {
                    use crate::accounts_snapshots::*;

                    #[derive(Arbitrary, Clone, DisplayIx, FuzzTestExecutor, FuzzDeserialize)]
                    pub enum FuzzInstruction {
                        #(#instructions),*
                    }

                    #(#instructions_data)*

                    #(#instructions_ixops_impls)*

                    // FIX this is just a workaround to propagate a comment to the source code easily
                    /// Use AccountsStorage<T> where T can be one of:
                    /// Keypair, PdaStore, TokenStore, MintStore, ProgramStore
                    #[derive(Default)]
                    pub struct FuzzAccounts {
                        #(#sorted_fuzz_accounts: AccountsStorage<todo!()>),*
                    }

                    impl FuzzAccounts {
                        pub fn new() -> Self {
                            Default::default()
                        }
                    }
                }
            };
            fuzzer_module.into_token_stream().to_string()
        })
        .collect::<String>();
    code
}
