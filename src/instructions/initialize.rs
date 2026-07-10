use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;
use pinocchio::cpi::{Seed, Signer};
use crate::state::Fundraiser;

pub fn process_initialize(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [maker, fundraiser, mint_to_raise, _vault, _token_program, _system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if data.len() < 10 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let amount_to_raise = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let duration = data[8];
    let bump = data[9];

    let id_bytes: [u8; 32] = crate::ID.as_ref().try_into().unwrap();
    let expected = derive_address(
        &[b"fundraiser", maker.address().as_ref()],
        Some(bump),
        &id_bytes,
    );
    if fundraiser.address().as_ref() != expected.as_slice() {
        return Err(ProgramError::InvalidSeeds);
    }

    let bump_arr = [bump];
    let seeds = [
        Seed::from(b"fundraiser".as_ref()),
        Seed::from(maker.address().as_ref()),
        Seed::from(bump_arr.as_ref()),
    ];
    let signer: Signer = (&seeds[..]).into();

    CreateAccount {
        from: maker,
        to: fundraiser,
        lamports: Rent::get()?.try_minimum_balance(90)?,
        space: 90,
        owner: &crate::ID,
    }
    .invoke_signed(&[signer])?;

    let clock = Clock::get()?;
    let mut fundraiser_data = fundraiser.try_borrow_mut()?;
    let state: &mut Fundraiser = unsafe { &mut *(fundraiser_data.as_mut_ptr() as *mut Fundraiser) };

    state.maker = maker.address().clone();
    state.mint_to_raise = mint_to_raise.address().clone();
    state.amount_to_raise = amount_to_raise;
    state.current_amount = 0;
    state.time_started = clock.unix_timestamp;
    state.duration = duration;
    state.bump = bump;

    Ok(())
}