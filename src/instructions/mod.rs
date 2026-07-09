use pinocchio::error::ProgramError;

pub enum FundraiserInstructions {
    Initialize,
        Contribute,
  CheckContributions,
          Refund,
                }

                impl TryFrom<&u8> for FundraiserInstructions {
               type Error = ProgramError;

       fn try_from(value: &u8) -> Result<Self, Self::Error> {
              match value {
       0 => Ok(FundraiserInstructions::Initialize),
         1 => Ok(FundraiserInstructions::Contribute),
          2 => Ok(FundraiserInstructions::CheckContributions),
             3 => Ok(FundraiserInstructions::Refund),
       _ => Err(ProgramError::InvalidInstructionData),
                                               }
                                          }
                                  }