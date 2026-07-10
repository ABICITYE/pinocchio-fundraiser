use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
};
use pinocchio_pubkey::derive_address;
use pinocchio_token::instructions::Transfer;
use pinocchio_system::instructions::CreateAccount;
use pinocchio::cpi::{Seed, Signer};
use crate::state::{Fundraiser, Contributor};

pub fn process_contribute(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [contributor, contributor_ata, fundraiser, vault, contributor_account, _mint_to_raise, _token_program, _system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !contributor.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if data.len() < 9 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let amount = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let ca_bump = data[8];

    let id_bytes: [u8; 32] = crate::ID.as_ref().try_into().unwrap();

    // --- READ-ONLY CHECKS FIRST (borrow, read, drop before any CPI) ---
    let (amount_to_raise, deadline) = {
        let fr_data = fundraiser.try_borrow()?;
        let fr_state: &Fundraiser = unsafe { &*(fr_data.as_ptr() as *const Fundraiser) };
        let clock = Clock::get()?;
        let deadline = fr_state.time_started + (fr_state.duration as i64) * 86_400;
        (fr_state.amount_to_raise, deadline)
    };

    let clock = Clock::get()?;
    if clock.unix_timestamp > deadline {
        return Err(ProgramError::Custom(1));
    }
    let max_contribution = amount_to_raise / 10;
    if amount > max_contribution {
        return Err(ProgramError::Custom(2));
    }

    let expected_ca = derive_address(
        &[b"contributor", fundraiser.address().as_ref(), contributor.address().as_ref()],
        Some(ca_bump),
        &id_bytes,
    );
    if contributor_account.address().as_ref() != expected_ca.as_slice() {
        return Err(ProgramError::InvalidSeeds);
    }

    // --- CPIs (no data borrows held across these) ---
    if contributor_account.lamports() == 0 {
        let bump_arr = [ca_bump];
        let seeds = [
            Seed::from(b"contributor".as_ref()),
            Seed::from(fundraiser.address().as_ref()),
            Seed::from(contributor.address().as_ref()),
            Seed::from(bump_arr.as_ref()),
        ];
        let signer: Signer = (&seeds[..]).into();

        CreateAccount {
            from: contributor,
            to: contributor_account,
            lamports: Rent::get()?.try_minimum_balance(8)?,
            space: 8,
            owner: &crate::ID,
        }
        .invoke_signed(&[signer])?;
    }

    Transfer {
        from: contributor_ata,
        to: vault,
        authority: contributor,
        amount,
    }
    .invoke()?;

    // --- WRITES LAST, fresh borrows after all CPIs are done ---
    let mut ca_data = contributor_account.try_borrow_mut()?;
    let ca_state: &mut Contributor = unsafe { &mut *(ca_data.as_mut_ptr() as *mut Contributor) };
    ca_state.amount = ca_state.amount.checked_add(amount).ok_or(ProgramError::ArithmeticOverflow)?;
    drop(ca_data);

    let mut fr_data = fundraiser.try_borrow_mut()?;
    let fr_state: &mut Fundraiser = unsafe { &mut *(fr_data.as_mut_ptr() as *mut Fundraiser) };
    fr_state.current_amount = fr_state.current_amount.checked_add(amount).ok_or(ProgramError::ArithmeticOverflow)?;

    Ok(())
}