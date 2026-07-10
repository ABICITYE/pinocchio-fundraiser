use litesvm::LiteSVM;
use litesvm_token::{CreateMint, CreateAssociatedTokenAccount, MintTo};
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_pubkey::Pubkey;
use solana_instruction::{Instruction, AccountMeta};
use solana_transaction::Transaction;
use solana_message::Message;

#[test]
fn test_full_fundraiser_flow() {
    let mut svm = LiteSVM::new();
    let program_id = Pubkey::new_from_array(escrow::ID.to_bytes());
    svm.add_program_from_file(program_id, "target/deploy/escrow.so").unwrap();

    let maker = Keypair::new();
    svm.airdrop(&maker.pubkey(), 10_000_000_000).unwrap();
    let contributor = Keypair::new();
    svm.airdrop(&contributor.pubkey(), 10_000_000_000).unwrap();

    let mint = CreateMint::new(&mut svm, &maker).decimals(6).send().unwrap();
    let contributor_ata = CreateAssociatedTokenAccount::new(&mut svm, &contributor, &mint).send().unwrap();
    MintTo::new(&mut svm, &maker, &mint, &contributor_ata, 1_000_000).send().unwrap();
    let maker_ata = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint).send().unwrap();

    let amount_to_raise: u64 = 100_000;
    let duration: u8 = 30;

    let maker_pubkey = maker.pubkey();
    let fr_seeds = &[b"fundraiser".as_ref(), maker_pubkey.as_ref()];
    let (fundraiser_pda, fr_bump) = Pubkey::find_program_address(fr_seeds, &program_id);

    let system_program = solana_pubkey::pubkey!("11111111111111111111111111111111111111");
    let spl_token_program = solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

    let vault = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint).owner(&fundraiser_pda).send().unwrap();

    // ---- STEP 1: initialize ----
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
    let r1 = svm.send_transaction(Transaction::new(&[&maker], Message::new(&[init_ix], Some(&maker.pubkey())), svm.latest_blockhash()));
    assert!(r1.is_ok(), "Step 1 (initialize) failed: {:?}", r1.err());
    println!("✅ Step 1: initialize succeeded");

    // ---- STEP 2: contribute (x10, to hit the 10% per-person cap and reach target) ----
    let contributor_pubkey = contributor.pubkey();
    let ca_seeds = &[b"contributor".as_ref(), fundraiser_pda.as_ref(), contributor_pubkey.as_ref()];
    let (contributor_account_pda, ca_bump) = Pubkey::find_program_address(ca_seeds, &program_id);

    for i in 0..10 {
        let contribute_amount: u64 = 10_000;
        let mut contrib_data = vec![1u8];
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
        svm.expire_blockhash();
        let r = svm.send_transaction(Transaction::new(&[&contributor], Message::new(&[contrib_ix], Some(&contributor.pubkey())), svm.latest_blockhash()));
        assert!(r.is_ok(), "Step 2 (contribute #{}) failed: {:?}", i, r.err());
    }
    println!("✅ Step 2: 10x contribute succeeded, target reached");

    // verify fundraiser state reflects all contributions
    let fr_account = svm.get_account(&fundraiser_pda).unwrap();
    let current_amount = u64::from_le_bytes(fr_account.data[72..80].try_into().unwrap());
    assert_eq!(current_amount, 100_000, "current_amount should equal target after 10 contributions");

    // ---- STEP 3: check_contributions (maker withdraws) ----
    let cc_data = vec![2u8, fr_bump];
    let cc_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(maker.pubkey(), true),
            AccountMeta::new(fundraiser_pda, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(maker_ata, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(spl_token_program, false),
        ],
        data: cc_data,
    };
    svm.expire_blockhash();
    let r3 = svm.send_transaction(Transaction::new(&[&maker], Message::new(&[cc_ix], Some(&maker.pubkey())), svm.latest_blockhash()));
    assert!(r3.is_ok(), "Step 3 (check_contributions) failed: {:?}", r3.err());
    println!("✅ Step 3: check_contributions succeeded, maker withdrew funds");

    // verify maker received all the funds
    let maker_ata_account = svm.get_account(&maker_ata).unwrap();
    let maker_balance = u64::from_le_bytes(maker_ata_account.data[64..72].try_into().unwrap());
    assert_eq!(maker_balance, 100_000, "maker should have received full target amount");

    println!("🎉 Full fundraiser flow verified end-to-end: initialize → contribute (x10) → check_contributions");
}