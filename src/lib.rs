#![allow(unexpected_cfgs)]
use pinocchio::{AccountView, entrypoint, Address, ProgramResult, address::declare_id, error::ProgramError};

mod instructions;
mod state;

use instructions::FundraiserInstructions;

entrypoint!(process_instruction);
declare_id!("7kP9ghngPnbNiRyFHwV3QVzfiHfaVw17ZECRoBBfkqPe");

pub fn process_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let (discriminator, data) = instruction_data.split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match FundraiserInstructions::try_from(discriminator)? {
        FundraiserInstructions::Initialize => instructions::process_initialize(accounts, data),
        FundraiserInstructions::Contribute => Ok(()),
        FundraiserInstructions::CheckContributions => Ok(()),
        FundraiserInstructions::Refund => Ok(()),
    }
}