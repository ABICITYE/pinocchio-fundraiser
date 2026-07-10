use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
};
use pinocchio_pubkey::derive_address;
use pinocchio_token::instructions::Transfer;
use pinocchio::cpi::{Seed, Signer};
use crate::state::Fundraiser;

pub fn process_check_contributions(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [maker, fundraiser, vault, maker_ata, _mint_to_raise, _token_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if data.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }
    let bump = data[0];

    let id_bytes: [u8; 32] = crate::ID.as_ref().try_into().unwrap();

    let (vault_amount, target_met, expected_maker) = {
        let fr_data = fundraiser.try_borrow()?;
        let fr_state: &Fundraiser = unsafe { &*(fr_data.as_ptr() as *const Fundraiser) };
        (fr_state.current_amount, fr_state.current_amount >= fr_state.amount_to_raise, fr_state.maker.clone())
    };

    if maker.address().as_ref() != expected_maker.as_ref() {
        return Err(ProgramError::IllegalOwner);
    }
    if !target_met {
        return Err(ProgramError::Custom(3)); // target not met
    }

    let expected_fundraiser = derive_address(
        &[b"fundraiser", maker.address().as_ref()],
        Some(bump),
        &id_bytes,
    );
    if fundraiser.address().as_ref() != expected_fundraiser.as_slice() {
        return Err(ProgramError::InvalidSeeds);
    }

    // PDA-signed transfer: vault -> maker
    let bump_arr = [bump];
    let seeds = [
        Seed::from(b"fundraiser".as_ref()),
        Seed::from(maker.address().as_ref()),
        Seed::from(bump_arr.as_ref()),
    ];
    let signer: Signer = (&seeds[..]).into();

    Transfer {
        from: vault,
        to: maker_ata,
        authority: fundraiser,
        amount: vault_amount,
    }
    .invoke_signed(&[signer])?;

    // close fundraiser: zero data, transfer lamports to maker, reassign owner to system program
    {
        let mut fr_data = fundraiser.try_borrow_mut()?;
        fr_data.fill(0);
    }
    let fundraiser_lamports = fundraiser.lamports();
    fundraiser.set_lamports(0);
    maker.set_lamports(maker.lamports() + fundraiser_lamports);

    Ok(())
}