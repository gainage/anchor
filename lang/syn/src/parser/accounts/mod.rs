use crate::{
    AccountField, AccountsStruct, CompositeField, Constraint, ConstraintGroup, CpiAccountTy,
    CpiStateTy, Field, LoaderTy, ProgramAccountTy, ProgramStateTy, SysvarTy, Ty,
};
use constraint::ConstraintGroupBuilder;
use syn::parse::{Error as ParseError, Result as ParseResult};
use syn::punctuated::Punctuated;
use syn::token::Comma;

pub mod constraint;

pub fn parse(strct: &syn::ItemStruct) -> ParseResult<AccountsStruct> {
    let fields = match &strct.fields {
        syn::Fields::Named(fields) => fields
            .named
            .iter()
            .map(parse_account_field)
            .collect::<ParseResult<Vec<AccountField>>>()?,
        _ => {
            return Err(ParseError::new_spanned(
                &strct.fields,
                "Accounts fields must be named",
            ))
        }
    };
    Ok(AccountsStruct::new(strct.clone(), fields))
}

pub fn parse_account_field(f: &syn::Field) -> ParseResult<AccountField> {
    let constraints = parse_constraints(f)?;

    let ident = f.ident.clone().unwrap();
    let account_field = match is_field_primitive(f) {
        true => {
            let ty = parse_ty(f);
            AccountField::Field(Field {
                ident,
                ty,
                constraints,
            })
        }
        false => AccountField::CompositeField(CompositeField {
            ident,
            symbol: ident_string(f),
            constraints,
            raw_field: f.clone(),
        }),
    };
    Ok(account_field)
}

pub fn parse_constraints(f: &syn::Field) -> ParseResult<ConstraintGroup> {
    let mut constraints = ConstraintGroupBuilder::default();
    for attr in f.attrs.iter().filter(is_account) {
        for c in attr.parse_args_with(Punctuated::<Constraint, Comma>::parse_terminated)? {
            constraints.add(c)?;
        }
    }
    constraints.build()
}

pub fn is_account(attr: &&syn::Attribute) -> bool {
    attr.path
        .get_ident()
        .map_or(false, |ident| ident == "account")
}

fn is_field_primitive(f: &syn::Field) -> bool {
    match ident_string(f).as_str() {
        "ProgramState" | "ProgramAccount" | "CpiAccount" | "Sysvar" | "AccountInfo"
        | "CpiState" | "Loader" => true,
        _ => false,
    }
}

fn parse_ty(f: &syn::Field) -> Ty {
    let path = match &f.ty {
        syn::Type::Path(ty_path) => ty_path.path.clone(),
        _ => panic!("invalid account syntax"),
    };
    match ident_string(f).as_str() {
        "ProgramState" => Ty::ProgramState(parse_program_state(&path)),
        "CpiState" => Ty::CpiState(parse_cpi_state(&path)),
        "ProgramAccount" => Ty::ProgramAccount(parse_program_account(&path)),
        "CpiAccount" => Ty::CpiAccount(parse_cpi_account(&path)),
        "Sysvar" => Ty::Sysvar(parse_sysvar(&path)),
        "AccountInfo" => Ty::AccountInfo,
        "Loader" => Ty::Loader(parse_program_account_zero_copy(&path)),
        _ => panic!("invalid account type"),
    }
}

fn ident_string(f: &syn::Field) -> String {
    let path = match &f.ty {
        syn::Type::Path(ty_path) => ty_path.path.clone(),
        _ => panic!("invalid account syntax"),
    };
    // TODO: allow segmented paths.
    assert!(path.segments.len() == 1);
    let segments = &path.segments[0];
    segments.ident.to_string()
}

fn parse_program_state(path: &syn::Path) -> ProgramStateTy {
    let account_ident = parse_account(&path);
    ProgramStateTy { account_ident }
}

fn parse_cpi_state(path: &syn::Path) -> CpiStateTy {
    let account_ident = parse_account(&path);
    CpiStateTy { account_ident }
}

fn parse_cpi_account(path: &syn::Path) -> CpiAccountTy {
    let account_ident = parse_account(path);
    CpiAccountTy { account_ident }
}

fn parse_program_account(path: &syn::Path) -> ProgramAccountTy {
    let account_ident = parse_account(path);
    ProgramAccountTy { account_ident }
}

fn parse_program_account_zero_copy(path: &syn::Path) -> LoaderTy {
    let account_ident = parse_account(path);
    LoaderTy { account_ident }
}

fn parse_account(path: &syn::Path) -> syn::Ident {
    let segments = &path.segments[0];
    match &segments.arguments {
        syn::PathArguments::AngleBracketed(args) => {
            // Expected: <'info, MyType>.
            assert!(args.args.len() == 2);
            match &args.args[1] {
                syn::GenericArgument::Type(syn::Type::Path(ty_path)) => {
                    // TODO: allow segmented paths.
                    assert!(ty_path.path.segments.len() == 1);
                    let path_segment = &ty_path.path.segments[0];
                    path_segment.ident.clone()
                }
                _ => panic!("Invalid ProgramAccount"),
            }
        }
        _ => panic!("Invalid ProgramAccount"),
    }
}

fn parse_sysvar(path: &syn::Path) -> SysvarTy {
    let segments = &path.segments[0];
    let account_ident = match &segments.arguments {
        syn::PathArguments::AngleBracketed(args) => {
            // Expected: <'info, MyType>.
            assert!(args.args.len() == 2);
            match &args.args[1] {
                syn::GenericArgument::Type(syn::Type::Path(ty_path)) => {
                    // TODO: allow segmented paths.
                    assert!(ty_path.path.segments.len() == 1);
                    let path_segment = &ty_path.path.segments[0];
                    path_segment.ident.clone()
                }
                _ => panic!("Invalid Sysvar"),
            }
        }
        _ => panic!("Invalid Sysvar"),
    };
    match account_ident.to_string().as_str() {
        "Clock" => SysvarTy::Clock,
        "Rent" => SysvarTy::Rent,
        "EpochSchedule" => SysvarTy::EpochSchedule,
        "Fees" => SysvarTy::Fees,
        "RecentBlockhashes" => SysvarTy::RecentBlockhashes,
        "SlotHashes" => SysvarTy::SlotHashes,
        "SlotHistory" => SysvarTy::SlotHistory,
        "StakeHistory" => SysvarTy::StakeHistory,
        "Instructions" => SysvarTy::Instructions,
        "Rewards" => SysvarTy::Rewards,
        _ => panic!("Invalid Sysvar"),
    }
}
