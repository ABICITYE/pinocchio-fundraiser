use litesvm::LiteSVM;
use litesvm_token::{CreateMint, CreateAssociatedTokenAccount, MintTo};
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_pubkey::Pubkey;
use solana_instruction::{Instruction, AccountMeta};
use solana_transaction::Transaction;
use solana_message::Message;
use solana_clock::Clock;

#[test]
fn test_refund() {
    let mut svm = LiteSVM::new();
    let program_id = Pubkey::new_from_array(escrow::ID.to_bytes());
    svm.add_program_from_file(program_id, "target/deploy/escrow.so").unwrap();

    let maker = Keypair::new();
    svm.airdrop(&maker.pubkey(), 10_000_000_000).unwrap();
    let contributor = Keypair::new();
    svm.airdrop(&contributor.pubkey(), 10_000_000_000).unwrap();

    let mint = CreateMint::new(&mut svm, &maker).decimals(6).send().unwrap();
    let contributor_ata = CreateAssociatedTokenAccount::new(&mut svm, &contributor, &mint).send().unwrap();
    MintTo::new(&mut svm, &maker, &mint, &contributor_ata, 500_000).send().unwrap();

    // large target so a small contribution won't meet it
    let amount_to_raise: u64 = 10_000_000;
    let duration: u8 = 1; // 1 day

    let maker_pubkey = maker.pubkey();
    let fr_seeds = &[b"fundraiser".as_ref(), maker_pubkey.as_ref()];
    let (fundraiser_pda, fr_bump) = Pubkey::find_program_address(fr_seeds, &program_id);

    let system_program = solana_pubkey::pubkey!("11111111111111111111111111111111111111");
    let spl_token_program = solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

    let vault = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint).owner(&fundraiser_pda).send().unwrap();

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
    svm.send_transaction(Transaction::new(&[&maker], Message::new(&[init_ix], Some(&maker.pubkey())), svm.latest_blockhash())).unwrap();

    // contribute a small amount (well under the target)
    let contributor_pubkey = contributor.pubkey();
    let ca_seeds = &[b"contributor".as_ref(), fundraiser_pda.as_ref(), contributor_pubkey.as_ref()];
    let (contributor_account_pda, ca_bump) = Pubkey::find_program_address(ca_seeds, &program_id);

    let contribute_amount: u64 = 100_000;
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
    svm.send_transaction(Transaction::new(&[&contributor], Message::new(&[contrib_ix], Some(&contributor.pubkey())), svm.latest_blockhash())).unwrap();

    // warp clock forward past the deadline (1 day = 86_400 seconds, warp 2 days to be safe)
    let mut clock: Clock = svm.get_sysvar();
    clock.unix_timestamp += 2 * 86_400;
    svm.set_sysvar::<Clock>(&clock);

    // now refund
    let refund_data = vec![3u8, fr_bump];
    let refund_ix = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(contributor.pubkey(), true),
            AccountMeta::new(contributor_ata, false),
            AccountMeta::new(fundraiser_pda, false),
            AccountMeta::new(vault, false),
            AccountMeta::new(contributor_account_pda, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(spl_token_program, false),
        ],
        data: refund_data,
    };
    svm.expire_blockhash();
    let result = svm.send_transaction(Transaction::new(&[&contributor], Message::new(&[refund_ix], Some(&contributor.pubkey())), svm.latest_blockhash()));
    assert!(result.is_ok(), "refund failed: {:?}", result.err());

    // verify contributor got their tokens back
    let contributor_ata_account = svm.get_account(&contributor_ata).unwrap();
    let balance = u64::from_le_bytes(contributor_ata_account.data[64..72].try_into().unwrap());
    assert_eq!(balance, 500_000); // full original balance restored
}