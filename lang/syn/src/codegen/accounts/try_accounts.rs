use crate::codegen::accounts::{constraints, generics};
use crate::{AccountField, AccountsStruct, Constraint, Field};
use quote::quote;

// Generates the `Accounts` trait implementation.
pub fn generate(accs: &AccountsStruct) -> proc_macro2::TokenStream {
    let name = &accs.ident;
    let (combined_generics, trait_generics, strct_generics) = generics(accs);

    // All fields without an `#[account(associated)]` attribute.
    let non_associated_fields: Vec<&AccountField> = accs
        .fields
        .iter()
        .filter(|af| !is_associated_init(af))
        .collect();

    // Deserialization for each field
    let deser_fields: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|af: &AccountField| {
            match af {
                AccountField::CompositeField(s) => {
                    let name = &s.ident;
                    let ty = &s.raw_field.ty;
                    quote! {
                        #[cfg(feature = "anchor-debug")]
                        ::solana_program::log::sol_log(stringify!(#name));
                        let #name: #ty = anchor_lang::Accounts::try_accounts(program_id, accounts)?;
                    }
                }
                AccountField::Field(f) => {
                    // Associated fields are *first* deserialized into
                    // AccountInfos, and then later deserialized into
                    // ProgramAccounts in the "constraint check" phase.
                    if is_associated_init(af) {
                        let name = &f.ident;
                        quote!{
                            let #name = &accounts[0];
                            *accounts = &accounts[1..];
                        }
                    } else {
                        let name = &f.typed_ident();
                        match f.is_init {
                            false => quote! {
                                #[cfg(feature = "anchor-debug")]
                                ::solana_program::log::sol_log(stringify!(#name));
                                let #name = anchor_lang::Accounts::try_accounts(program_id, accounts)?;
                            },
                            true => quote! {
                                #[cfg(feature = "anchor-debug")]
                                ::solana_program::log::sol_log(stringify!(#name));
                                let #name = anchor_lang::AccountsInit::try_accounts_init(program_id, accounts)?;
                            },
                        }
                    }
                }
            }
        })
        .collect();

    // Deserialization for each *associated* field. This must be after
    // the deser_fields.
    let deser_associated_fields: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .filter_map(|af| match af {
            AccountField::CompositeField(_s) => None,
            AccountField::Field(f) => match is_associated_init(af) {
                false => None,
                true => Some(f),
            },
        })
        .map(|field: &Field| {
            // TODO: the constraints should be sorted so that the associated
            //       constraint comes first.
            let checks = field
                .constraints
                .iter()
                .map(|c| constraints::generate(&field, c))
                .collect::<Vec<proc_macro2::TokenStream>>();
            quote! {
                #(#checks)*
            }
        })
        .collect();

    // Constraint checks for each account fields.
    let access_checks: Vec<proc_macro2::TokenStream> = non_associated_fields
        .iter()
        .map(|af: &&AccountField| {
            let checks: Vec<proc_macro2::TokenStream> = match af {
                AccountField::Field(f) => f
                    .constraints
                    .iter()
                    .map(|c| constraints::generate(&f, c))
                    .collect(),
                AccountField::CompositeField(s) => s
                    .constraints
                    .iter()
                    .map(|c| constraints::generate_composite(&s, c))
                    .collect(),
            };
            quote! {
                #(#checks)*
            }
        })
        .collect();

    // Each field in the final deserialized accounts struct.
    let return_tys: Vec<proc_macro2::TokenStream> = accs
        .fields
        .iter()
        .map(|f: &AccountField| {
            let name = match f {
                AccountField::CompositeField(s) => &s.ident,
                AccountField::Field(f) => &f.ident,
            };
            quote! {
                #name
            }
        })
        .collect();
    quote! {
        impl#combined_generics anchor_lang::Accounts#trait_generics for #name#strct_generics {
            #[inline(never)]
            fn try_accounts(
                program_id: &anchor_lang::solana_program::pubkey::Pubkey,
                accounts: &mut &[anchor_lang::solana_program::account_info::AccountInfo<'info>],
            ) -> std::result::Result<Self, anchor_lang::solana_program::program_error::ProgramError> {
                // Deserialize each account.
                #(#deser_fields)*
                // Deserialize each associated account.
                //
                // Associated accounts are treated specially, because the fields
                // do deserialization + constraint checks in a single go,
                // whereas all other fields, i.e. the `deser_fields`, first
                // deserialize, and then do constraint checks.
                #(#deser_associated_fields)*
                // Perform constraint checks on each account.
                #(#access_checks)*
                // Success. Return the validated accounts.
                Ok(#name {
                    #(#return_tys),*
                })
            }
        }
    }
}

// Returns true if the given AccountField has an associated init constraint.
fn is_associated_init(af: &AccountField) -> bool {
    match af {
        AccountField::CompositeField(_s) => false,
        AccountField::Field(f) => f
            .constraints
            .iter()
            .filter(|c| match c {
                Constraint::Associated(c) => c.is_init,
                _ => false,
            })
            .next()
            .is_some(),
    }
}
