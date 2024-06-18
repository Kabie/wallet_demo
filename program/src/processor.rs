use borsh::BorshDeserialize;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::invoke_signed,
    pubkey::Pubkey,
    system_program,
};

use crate::assertions::{assert_pda, assert_same_pubkeys, assert_signer, assert_writable};
use crate::error::WalletError;
use crate::instruction::accounts::{CreateAccounts, ProxyCallAccounts};
use crate::instruction::WalletInstruction;
use crate::state::{Key, WalletAccount};
use crate::utils::create_account;

pub fn process_instruction<'a>(
    _program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    mut instruction_data: &[u8],
) -> ProgramResult {
    let instruction = WalletInstruction::deserialize(&mut instruction_data)?;
    match instruction {
        WalletInstruction::Create => {
            if !instruction_data.is_empty() {
                return Err(WalletError::DeserializationError.into());
            }
            msg!("Ix: Create");
            create(accounts)
        }
        WalletInstruction::ProxyCall => {
            msg!("Ix: ProxyCall");
            proxy_call(accounts, instruction_data)
        }
    }
}

fn create<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    // Accounts.
    let ctx = CreateAccounts::context(accounts)?;

    // Guards.
    let mut seeds = WalletAccount::seeds(ctx.accounts.authority.key);
    let wallet_bump = assert_pda(
        "wallet",
        ctx.accounts.wallet,
        &crate::ID,
        &seeds,
    )?;
    let vault_bump = assert_pda(
        "vault",
        ctx.accounts.vault,
        &crate::ID,
        &[b"VAULT", ctx.accounts.wallet.key.as_ref()],
    )?;
    assert_signer("authority", ctx.accounts.authority)?;
    assert_signer("payer", ctx.accounts.payer)?;
    assert_writable("payer", ctx.accounts.payer)?;
    assert_same_pubkeys(
        "system_program",
        ctx.accounts.system_program,
        &system_program::id(),
    )?;

    // Do nothing if the domain already exists.
    if !ctx.accounts.wallet.data_is_empty() {
        return Ok(());
    }

    // Create Counter PDA.
    let wallet = WalletAccount {
        key: Key::Wallet,
        authority: *ctx.accounts.authority.key,
        vault: *ctx.accounts.vault.key,
        vault_bump,
    };
    let bump = [wallet_bump];
    seeds.push(&bump);
    create_account(
        ctx.accounts.wallet,
        ctx.accounts.payer,
        ctx.accounts.system_program,
        WalletAccount::LEN,
        &crate::ID,
        Some(&[&seeds]),
    )?;

    wallet.save(ctx.accounts.wallet)
}

fn proxy_call<'a>(accounts: &'a [AccountInfo<'a>], instruction_data: &[u8]) -> ProgramResult {
    // Accounts.
    let ctx = ProxyCallAccounts::context(accounts)?;
    let wallet_account = WalletAccount::load(ctx.accounts.wallet)?;

    let mut wallet_seed = WalletAccount::seeds(&wallet_account.authority);
    let bump = [wallet_account.vault_bump];
    wallet_seed.push(&bump);

    let vault_seeds = [b"VAULT", ctx.accounts.wallet.key.as_ref(), &[wallet_account.vault_bump]];

    let ix_accounts = ctx
        .remaining_accounts
        .iter()
        .map(|a| {
            if a.key == ctx.accounts.vault.key {
                AccountMeta::new(*a.key, true)
            } else {
                AccountMeta::new(*a.key, a.is_signer)
            }
}       )
        .collect();
    let instruction = Instruction::new_with_bytes(
        *ctx.accounts.target_program.key,
        instruction_data,
        ix_accounts,
    );

    msg!("ProxyCall: {:?}", instruction);

    invoke_signed(&instruction, ctx.remaining_accounts, &[&vault_seeds])
}
