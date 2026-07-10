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

    // data: amount (8 bytes) + contributor_account_bump (1 byte)
    if data.len() < 9 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let amount = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let ca_bump = data[8];

    let id_bytes: [u8; 32] = crate::ID.as_ref().try_into().unwrap();

    // read fundraiser state
    let mut fr_data = fundraiser.try_borrow_mut()?;
    let fr_state: &mut Fundraiser = unsafe { &mut *(fr_data.as_mut_ptr() as *mut Fundraiser) };

    // check deadline: time_started + duration days must not have passed
    let clock = Clock::get()?;
    let deadline = fr_state.time_started + (fr_state.duration as i64) * 86_400;
    if clock.unix_timestamp > deadline {
        return Err(ProgramError::Custom(1)); // fundraiser expired
    }

    // check per-person cap: 10% of amount_to_raise
    let max_contribution = fr_state.amount_to_raise / 10;
    if amount > max_contribution {
        return Err(ProgramError::Custom(2)); // exceeds max contribution
    }

    // verify contributor PDA
    let expected_ca = derive_address(
        &[b"contributor", fundraiser.address().as_ref(), contributor.address().as_ref()],
        Some(ca_bump),
        &id_bytes,
    );
    if contributor_account.address().as_ref() != expected_ca.as_slice() {
        return Err(ProgramError::InvalidSeeds);
    }

    // create contributor_account if it doesn't exist yet (lamports == 0 means uninitialized)
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

    // transfer tokens contributor -> vault
    Transfer {
        from: contributor_ata,
        to: vault,
        authority: contributor,
        amount,
    }
    .invoke()?;

    // update contributor's tracked amount
    let mut ca_data = contributor_account.try_borrow_mut()?;
    let ca_state: &mut Contributor = unsafe { &mut *(ca_data.as_mut_ptr() as *mut Contributor) };
    ca_state.amount = ca_state.amount.checked_add(amount).ok_or(ProgramError::ArithmeticOverflow)?;

    // update fundraiser's current_amount
    fr_state.current_amount = fr_state.current_amount.checked_add(amount).ok_or(ProgramError::ArithmeticOverflow)?;

    Ok(())
}