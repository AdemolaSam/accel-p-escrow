use crate::{ProgramError::NotEnoughAccountKeys, error::EscrowError, state::Escrow};
use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError::{self},
};
use pinocchio_pubkey::derive_address;
use pinocchio_token::instructions::Transfer;

pub fn process_take_instruction(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [
        taker,
        maker,
        mint_a,
        mint_b,
        escrow_account,
        maker_ata_b,
        taker_ata_a,
        taker_ata_b,
        escrow_ata,
        token_program,
        _associated_token_program @ ..,
    ] = accounts
    else {
        return Err(NotEnoughAccountKeys);
    };

    let bump = data[0];
    let seed = [b"escrow".as_ref(), maker.address().as_ref(), &[bump]];

    let escrow_account_pda = derive_address(&seed, None, &crate::ID.to_bytes());
    assert_eq!(escrow_account_pda, *escrow_account.address().as_array());

    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"escrow"),
        Seed::from(maker.address().as_array()),
        Seed::from(bump_bytes.as_ref()),
    ];
    let signer = Signer::from(&signer_seeds);

    let (mint_b_expected, amount_to_receive, amount_to_give, escrow_maker) = {
        let escrow_state = Escrow::from_account_info(escrow_account)?;
        (
            escrow_state.mint_b(),
            escrow_state.amount_to_receive(),
            escrow_state.amount_to_give(),
            escrow_state.maker(),
        )
    };

    //CHECK IF:
    //escrow_ata is owned by the token_program
    if !escrow_ata.owned_by(token_program.address()) {
        return Err(ProgramError::IncorrectProgramId);
    }

    // correct mint
    if mint_b.address() != mint_b_expected {
        return Err(EscrowError::InvalidToken.into());
    }

    //mint is owned by
    if !mint_a.owned_by(token_program.address()) {
        return Err(ProgramError::IncorrectProgramId);
    }
    if !mint_b.owned_by(token_program.address()) {
        return Err(ProgramError::IncorrectProgramId);
    }

    //maker owns the escrow
    if maker.address() != escrow_maker {
        return Err(ProgramError::IllegalOwner);
    }
    //maker owns the ata for token b
    {
        let maker_ata_state = pinocchio_token::state::Account::from_account_view(maker_ata_b)?;
        if maker_ata_state.owner() != maker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if maker_ata_state.mint() != mint_b.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }
    //taker owns the atas
    {
        let taker_ata_state = pinocchio_token::state::Account::from_account_view(taker_ata_a)?;
        if taker_ata_state.owner() != taker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if taker_ata_state.mint() != mint_a.address() {
            return Err(ProgramError::InvalidAccountData);
        }

        let taker_ata_b_state = pinocchio_token::state::Account::from_account_view(taker_ata_b)?;
        if taker_ata_b_state.owner() != taker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if taker_ata_b_state.mint() != mint_b.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }
    // escrow ata mint is mint_a
    {
        let escrow_ata_state = pinocchio_token::state::Account::from_account_view(escrow_ata)?;
        if escrow_ata_state.mint() != mint_a.address() {
            return Err(ProgramError::InvalidAccountData);
        }
        if escrow_ata_state.owner() != escrow_account.address() {
            return Err(ProgramError::IllegalOwner);
        }
    }

    //deposit tokens to maker_ata
    Transfer::new(taker_ata_b, maker_ata_b, taker, amount_to_receive).invoke()?;

    Transfer::new(escrow_ata, taker_ata_a, escrow_account, amount_to_give)
        .invoke_signed(&[signer])?;

    //close escrow ata/vault account
    pinocchio_token::instructions::CloseAccount::new(escrow_ata, maker, escrow_account)
        .invoke_signed(&[Signer::from(&signer_seeds)])?;

    Escrow::close_pda(escrow_account, maker)?;

    Ok(())
}
