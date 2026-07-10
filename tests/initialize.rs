use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_pubkey::Pubkey;
use solana_instruction::{Instruction, AccountMeta};
use solana_transaction::Transaction;
use solana_message::Message;

#[test]
fn test_initialize() {
    let mut svm = LiteSVM::new();
    let program_id = Pubkey::new_from_array(escrow::ID.to_bytes());
    svm.add_program_from_file(program_id, "target/deploy/escrow.so").unwrap();

    let maker = Keypair::new();
    svm.airdrop(&maker.pubkey(), 10_000_000_000).unwrap();

    let mint_to_raise = Pubkey::new_unique(); // placeholder mint for now

    let amount_to_raise: u64 = 1_000_000;
    let duration: u8 = 30;

    let maker_pubkey = maker.pubkey();
let seeds = &[b"fundraiser".as_ref(), maker_pubkey.as_ref()];
let (fundraiser_pda, bump) = Pubkey::find_program_address(seeds, &program_id);

    let mut data = vec![0u8]; // discriminator 0 = Initialize
    data.extend_from_slice(&amount_to_raise.to_le_bytes());
    data.push(duration);
    data.push(bump);

    let vault = Pubkey::new_unique(); // placeholder for now
    let token_program = Pubkey::new_unique();
    let system_program = solana_pubkey::pubkey!("11111111111111111111111111111111111111");

    let ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(maker.pubkey(), true),
            AccountMeta::new(fundraiser_pda, false),
            AccountMeta::new_readonly(mint_to_raise, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(token_program, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    };

    let tx = Transaction::new(
        &[&maker],
        Message::new(&[ix], Some(&maker.pubkey())),
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "initialize failed: {:?}", result.err());

    let account = svm.get_account(&fundraiser_pda).unwrap();
    assert_eq!(account.data.len(), 90);
    assert_eq!(&account.data[0..32], maker.pubkey().as_ref());
}