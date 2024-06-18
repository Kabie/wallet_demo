use dephy_io_wallet_demo::{instruction::WalletInstruction, state::WalletAccount, utils::find_vault_pda};
use solana_program_test::{tokio, ProgramTest, ProgramTestContext};
use solana_sdk::{
    instruction::{AccountMeta, Instruction as SolanaInstruction}, native_token::sol_to_lamports, pubkey::Pubkey, signature::Keypair, signer::Signer, system_instruction, system_program, transaction::Transaction
};

#[tokio::test]
async fn test_wallet() {
    let program_id = dephy_io_wallet_demo::id();

    let program_test = ProgramTest::new("dephy_io_wallet_demo", program_id, None);

    let mut ctx = program_test.start_with_context().await;

    let authority = Keypair::new();
    let (wallet_pubkey, _) = WalletAccount::find_pda(&authority.pubkey());
    let (vault_pubkey, _vault_bump) = find_vault_pda(&wallet_pubkey);

    // test create
    let mut transaction = Transaction::new_with_payer(
        &[SolanaInstruction::new_with_borsh(
            program_id,
            &WalletInstruction::Create,
            vec![
                // #[account(0, writable, name="wallet", desc = "The program derived address of the wallet account to create (seeds: ['WALLET', authority])")]
                AccountMeta::new(wallet_pubkey, false),
                // #[account(1, signer, name="authority", desc = "The authority of the wallet")]
                AccountMeta::new(authority.pubkey(), true),
                // #[account(2, name="vault", desc = "The wallet vault (seeds: ['VAULT', wallet])")]
                AccountMeta::new(vault_pubkey, false),
                // #[account(3, writable, signer, name="payer", desc = "The account paying for the storage fees")]
                AccountMeta::new(ctx.payer.pubkey(), true),
                // #[account(4, name="system_program", desc = "The system program")]
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        Some(&ctx.payer.pubkey()),
    );
    transaction.sign(&[&ctx.payer, &authority], ctx.last_blockhash);
    ctx.banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Associated account now exists
    let demo_account = ctx
        .banks_client
        .get_account(wallet_pubkey)
        .await
        .expect("get_account")
        .expect("Account not none");
    assert_eq!(demo_account.data.len(), WalletAccount::LEN);

    airdrop(&mut ctx, &vault_pubkey, sol_to_lamports(10.0)).await;
    assert_eq!(ctx.banks_client.get_balance(vault_pubkey).await.unwrap(), sol_to_lamports(10.0));

    let dest_account = Pubkey::new_unique();
    let mut ix = system_instruction::transfer(&vault_pubkey, &dest_account, sol_to_lamports(1.0));
    test_proxy_call(&mut ctx, &program_id, &authority, &mut ix).await;
    assert_eq!(ctx.banks_client.get_balance(vault_pubkey).await.unwrap(), sol_to_lamports(9.0));
}

async fn airdrop(ctx: &mut ProgramTestContext, to_account: &Pubkey, lamports: u64) {
    let mut transaction = Transaction::new_with_payer(
        &[system_instruction::transfer(&ctx.payer.pubkey(), &to_account, lamports)],
        Some(&ctx.payer.pubkey()),
    );
    transaction.sign(&[&ctx.payer], ctx.last_blockhash);
    ctx.banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

async fn test_proxy_call(ctx: &mut ProgramTestContext, program_id: &Pubkey, authority: &Keypair, inner_ix: &mut SolanaInstruction) {
    let (wallet_pubkey, _) = WalletAccount::find_pda(&authority.pubkey());
    let (vault_pubkey, _vault_bump) = find_vault_pda(&wallet_pubkey);

    let mut accounts = vec![
        // #[account(0, name="wallet", desc = "The program derived address of the wallet account to increment (seeds: ['WALLET', authority])")]
        AccountMeta::new(wallet_pubkey, false),
        // #[account(1, signer, name="authority", desc = "The authority of the wallet")]
        AccountMeta::new(authority.pubkey(), true),
        // #[account(2, name="vault", desc = "The wallet vault (seeds: ['VAULT', wallet])")]
        AccountMeta::new(vault_pubkey, false),
        // #[account(3, name="target_program", desc = "The proxy called program")]
        AccountMeta::new(inner_ix.program_id, false),
    ];

    inner_ix.accounts.iter().for_each(|a| {
        if a.pubkey == vault_pubkey {
            accounts.push(AccountMeta::new(a.pubkey, false));
        } else {
            accounts.push(a.to_owned());
        }
    });

    // accounts
    let mut data = borsh::to_vec(&WalletInstruction::ProxyCall).unwrap();
    data.append(&mut inner_ix.data);

    let mut transaction = Transaction::new_with_payer(
        &[SolanaInstruction::new_with_bytes(
            *program_id,
            &data,
            accounts,
        )],
        Some(&ctx.payer.pubkey()),
    );

    transaction.sign(&[&authority, &ctx.payer], ctx.last_blockhash);
    ctx.banks_client
        .process_transaction(transaction)
        .await
        .unwrap()
}

