use litesvm::LiteSVM;
use litesvm_token::{CreateMint, CreateAssociatedTokenAccount, MintTo};
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_pubkey::Pubkey;
use solana_instruction::{Instruction, AccountMeta};
use solana_transaction::Transaction;
use solana_message::Message;

#[test]
fn test_contribute() {
    let mut svm = LiteSVM::new();
    let program_id = Pubkey::new_from_array(escrow::ID.to_bytes());
    svm.add_program_from_file(program_id, "target/deploy/escrow.so").unwrap();

    let maker = Keypair::new();
    svm.airdrop(&maker.pubkey(), 10_000_000_000).unwrap();

    let contributor = Keypair::new();
    svm.airdrop(&contributor.pubkey(), 10_000_000_000).unwrap();

    // create the mint
    let mint = CreateMint::new(&mut svm, &maker).decimals(6).send().unwrap();

    // create contributor's token account and mint them tokens
    let contributor_ata = CreateAssociatedTokenAccount::new(&mut svm, &contributor, &mint)
        .send()
        .unwrap();
    MintTo::new(&mut svm, &maker, &mint, &contributor_ata, 500_000).send().unwrap();

    let amount_to_raise: u64 = 1_000_000;
    let duration: u8 = 30;

    let maker_pubkey = maker.pubkey();
    let fr_seeds = &[b"fundraiser".as_ref(), maker_pubkey.as_ref()];
    let (fundraiser_pda, fr_bump) = Pubkey::find_program_address(fr_seeds, &program_id);

    let system_program = solana_pubkey::pubkey!("11111111111111111111111111111111111111");
    let spl_token_program = solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

    // --- run initialize first ---
    let vault = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint)
        .owner(&fundraiser_pda)
        .send()
        .unwrap();

    let mut init_data = vec![0u8];
    init_data.extend_from_slice(&amount_to_raise.to_le_bytes());
    init_data.push(duration);
    init_data.push(fr_bump);

    let init_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(maker.pubkey(), true),
            AccountMeta::new(fundraiser_pda, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(spl_token_program, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data: init_data,
    };
    let init_tx = Transaction::new(&[&maker], Message::new(&[init_ix], Some(&maker.pubkey())), svm.latest_blockhash());
    svm.send_transaction(init_tx).unwrap();

    // --- now contribute ---
    let contributor_pubkey = contributor.pubkey();
    let ca_seeds = &[b"contributor".as_ref(), fundraiser_pda.as_ref(), contributor_pubkey.as_ref()];
    let (contributor_account_pda, ca_bump) = Pubkey::find_program_address(ca_seeds, &program_id);

    let contribute_amount: u64 = 50_000;
    let mut contrib_data = vec![1u8]; // discriminator 1 = Contribute
    contrib_data.extend_from_slice(&contribute_amount.to_le_bytes());
    contrib_data.push(ca_bump);

    let contrib_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(contributor.pubkey(), true),
            AccountMeta::new(contributor_ata, false),
            AccountMeta::new(fundraiser_pda, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(contributor_account_pda, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(spl_token_program, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data: contrib_data,
    };
    let contrib_tx = Transaction::new(&[&contributor], Message::new(&[contrib_ix], Some(&contributor.pubkey())), svm.latest_blockhash());
    let result = svm.send_transaction(contrib_tx);
    assert!(result.is_ok(), "contribute failed: {:?}", result.err());

    // verify fundraiser's current_amount increased
    let fr_account = svm.get_account(&fundraiser_pda).unwrap();
    let current_amount = u64::from_le_bytes(fr_account.data[72..80].try_into().unwrap());
    assert_eq!(current_amount, contribute_amount);
}