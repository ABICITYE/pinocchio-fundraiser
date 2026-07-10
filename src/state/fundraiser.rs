use pinocchio::Address;

#[repr(C)]
pub struct Fundraiser {
    pub maker: Address,
    pub mint_to_raise: Address,
    pub amount_to_raise: u64,
    pub current_amount: u64,
    pub time_started: i64,
    pub duration: u8,
    pub bump: u8,
}