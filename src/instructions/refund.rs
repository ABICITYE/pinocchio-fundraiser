use pinocchio::{
    AccountView, ProgramResult,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
};
use pinocchio_token::instructions::Transfer;
use pinocchio::cpi::{Seed, Signer};
use crate::state::{Fundraiser, Contributor};

pub fn process_refund(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let [contributor, contributor_ata, fundraiser, vault, contributor_account, _mint_to_raise, _token_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !contributor.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if data.is_empty() {
        return Err(ProgramError::InvalidInstructionData);
    }
    let fr_bump = data[0];

    let (deadline, target_met, maker_key, refund_amount) = {
        let fr_data = fundraiser.try_borrow()?;
        let fr_state: &Fundraiser = unsafe { &*(fr_data.as_ptr() as *const Fundraiser) };
        let deadline = fr_state.time_started + (fr_state.duration as i64) * 86_400;
        let target_met = fr_state.current_amount >= fr_state.amount_to_raise;
        (deadline, target_met, fr_state.maker.clone(), fr_state.current_amount)
    };

    let clock = Clock::get()?;
    if clock.unix_timestamp <= deadline {
        return Err(ProgramError::Custom(4)); // fundraiser still active
    }
    if target_met {
        return Err(ProgramError::Custom(5)); // target was met, no refunds
    }

    let ca_amount = {
        let ca_data = contributor_account.try_borrow()?;
        let ca_state: &Contributor = unsafe { &*(ca_data.as_ptr() as *const Contributor) };
        ca_state.amount
    };

    // PDA-signed transfer: vault -> contributor
    let bump_arr = [fr_bump];
    let seeds = [
        Seed::from(b"fundraiser".as_ref()),
        Seed::from(maker_key.as_ref()),
        Seed::from(bump_arr.as_ref()),
    ];
    let signer: Signer = (&seeds[..]).into();

    Transfer {
        from: vault,
        to: contributor_ata,
        authority: fundraiser,
        amount: ca_amount,
    }
    .invoke_signed(&[signer])?;

    // close contributor_account: zero data, return lamports to contributor
    {
        let mut ca_data = contributor_account.try_borrow_mut()?;
        ca_data.fill(0);
    }
    let ca_lamports = contributor_account.lamports();
    contributor_account.set_lamports(0);
    contributor.set_lamports(contributor.lamports() + ca_lamports);

    // decrement fundraiser's current_amount
    let mut fr_data = fundraiser.try_borrow_mut()?;
    let fr_state: &mut Fundraiser = unsafe { &mut *(fr_data.as_mut_ptr() as *mut Fundraiser) };
    fr_state.current_amount = fr_state.current_amount.checked_sub(ca_amount).ok_or(ProgramError::ArithmeticOverflow)?;

    let _ = refund_amount; // unused, kept for clarity

    Ok(())
}