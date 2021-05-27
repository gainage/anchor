use crate::{
    Constraint, ConstraintAssociated, ConstraintBelongsTo, ConstraintExecutable, ConstraintLiteral,
    ConstraintOwner, ConstraintRentExempt, ConstraintSeeds, ConstraintSigner, ConstraintState,
};

pub fn parse(
    anchor: &syn::Attribute,
) -> (
    Vec<Constraint>,
    bool,
    bool,
    bool,
    Option<syn::Ident>,
    Option<proc_macro2::TokenStream>,
    Vec<syn::Ident>,
) {
    let mut tts = anchor.tokens.clone().into_iter();
    let g_stream = match tts.next().expect("Must have a token group") {
        proc_macro2::TokenTree::Group(g) => g.stream(),
        _ => panic!("Invalid syntax"),
    };

    let mut is_init = false;
    let mut is_mut = false;
    let mut is_signer = false;
    let mut constraints = vec![];
    let mut is_rent_exempt = None;
    let mut payer = None;
    let mut space = None;
    let mut is_associated = false;
    let mut associated_seeds = Vec::new();

    let mut inner_tts = g_stream.into_iter();
    while let Some(token) = inner_tts.next() {
        match token {
            proc_macro2::TokenTree::Ident(ident) => match ident.to_string().as_str() {
                "init" => {
                    is_init = true;
                    is_mut = true;
                    // If it's not specified, all program owned accounts default
                    // to being rent exempt.
                    if is_rent_exempt.is_none() {
                        is_rent_exempt = Some(true);
                    }
                }
                "mut" => {
                    is_mut = true;
                }
                "signer" => {
                    is_signer = true;
                    constraints.push(Constraint::Signer(ConstraintSigner {}));
                }
                "seeds" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    let seeds = match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Group(g) => g,
                        _ => panic!("invalid syntax"),
                    };
                    constraints.push(Constraint::Seeds(ConstraintSeeds { seeds }))
                }
                "belongs_to" | "has_one" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    let join_target = match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Ident(ident) => ident,
                        _ => panic!("invalid syntax"),
                    };
                    constraints.push(Constraint::BelongsTo(ConstraintBelongsTo { join_target }))
                }
                "owner" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    let owner_target = match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Ident(ident) => ident,
                        _ => panic!("invalid syntax"),
                    };
                    constraints.push(Constraint::Owner(ConstraintOwner { owner_target }));
                }
                "rent_exempt" => {
                    match inner_tts.next() {
                        None => is_rent_exempt = Some(true),
                        Some(tkn) => {
                            match tkn {
                                proc_macro2::TokenTree::Punct(punct) => {
                                    assert!(punct.as_char() == '=');
                                    punct
                                }
                                _ => panic!("invalid syntax"),
                            };
                            let should_skip = match inner_tts.next().unwrap() {
                                proc_macro2::TokenTree::Ident(ident) => ident,
                                _ => panic!("invalid syntax"),
                            };
                            match should_skip.to_string().as_str() {
                                "skip" => {
                                    is_rent_exempt = Some(false);
                                },
                                _ => panic!("invalid syntax: omit the rent_exempt attribute to enforce rent exemption"),
                            };
                        }
                    };
                }
                "executable" => {
                    constraints.push(Constraint::Executable(ConstraintExecutable {}));
                }
                "state" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    let program_target = match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Ident(ident) => ident,
                        _ => panic!("invalid syntax"),
                    };
                    constraints.push(Constraint::State(ConstraintState { program_target }));
                }
                "associated" => {
                    is_associated = true;
                    is_mut = true;
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    let associated_target = match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Ident(ident) => ident,
                        _ => panic!("invalid syntax"),
                    };
                    constraints.push(Constraint::Associated(ConstraintAssociated {
                        associated_target,
                        is_init,
                    }));
                }
                "with" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    associated_seeds.push(match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Ident(ident) => ident,
                        _ => panic!("invalid syntax"),
                    });
                }
                "payer" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    let _payer = match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Ident(ident) => ident,
                        _ => panic!("invalid syntax"),
                    };
                    payer = Some(_payer);
                }
                "space" => {
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Punct(punct) => {
                            assert!(punct.as_char() == '=');
                            punct
                        }
                        _ => panic!("invalid syntax"),
                    };
                    match inner_tts.next().unwrap() {
                        proc_macro2::TokenTree::Literal(literal) => {
                            let tokens: proc_macro2::TokenStream =
                                literal.to_string().replace("\"", "").parse().unwrap();
                            space = Some(tokens);
                        }
                        _ => panic!("invalid space"),
                    }
                }
                _ => {
                    panic!("invalid syntax");
                }
            },
            proc_macro2::TokenTree::Punct(punct) => {
                if punct.as_char() != ',' {
                    panic!("invalid syntax");
                }
            }
            proc_macro2::TokenTree::Literal(literal) => {
                let tokens: proc_macro2::TokenStream =
                    literal.to_string().replace("\"", "").parse().unwrap();
                constraints.push(Constraint::Literal(ConstraintLiteral { tokens }));
            }
            _ => {
                panic!("invalid syntax");
            }
        }
    }

    // If init, then tag the associated constraint as being part of init.
    if is_init {
        for c in &mut constraints {
            if let Constraint::Associated(ConstraintAssociated { is_init, .. }) = c {
                *is_init = true;
            }
        }
    }

    // If `associated` is given, remove `init` since it's redundant.
    if is_associated {
        is_init = false;
    }

    if let Some(is_re) = is_rent_exempt {
        match is_re {
            false => constraints.push(Constraint::RentExempt(ConstraintRentExempt::Skip)),
            true => constraints.push(Constraint::RentExempt(ConstraintRentExempt::Enforce)),
        }
    }

    (
        constraints,
        is_mut,
        is_signer,
        is_init,
        payer,
        space,
        associated_seeds,
    )
}
