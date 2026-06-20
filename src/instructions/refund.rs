use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
};
use pinocchio_pubkey::derive_address;
use pinocchio_token::instructions::Transfer;

use crate::state::Escrow;

pub fn process_refund_instruction(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [
        maker,
        mint_a,
        escrow_account,
        maker_ata,
        escrow_ata,
        _associated_token_program @ ..,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    {
        let maker_ata_state = pinocchio_token::state::Account::from_account_view(maker_ata)?;
        if maker_ata_state.owner() != maker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if maker_ata_state.mint() != mint_a.address() {
            return Err(ProgramError::InvalidAccountData);
        }

        if !escrow_account.owned_by(&crate::ID) {
            return Err(ProgramError::IllegalOwner);
        }
    }

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

    if !escrow_account.owned_by(&crate::ID) {
        return Err(ProgramError::IllegalOwner);
    }

    let refund_amount = {
        let escrow_ata_state = pinocchio_token::state::Account::from_account_view(escrow_ata)?;
        escrow_ata_state.amount()
    };

    Transfer::new(escrow_ata, maker_ata, escrow_account, refund_amount).invoke_signed(&[signer])?;

    pinocchio_token::instructions::CloseAccount::new(escrow_ata, maker, escrow_account)
        .invoke_signed(&[Signer::from(&signer_seeds)])?;

    Escrow::close_pda(escrow_account, maker)?;

    Ok(())
}
