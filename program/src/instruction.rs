use borsh::{BorshDeserialize, BorshSerialize};
use shank::{ShankContext, ShankInstruction};

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, ShankContext, ShankInstruction)]
#[rustfmt::skip]
pub enum WalletInstruction {
    /// Creates the wallet account derived from the provided authority.
    #[account(0, writable, name="wallet", desc = "The program derived address of the wallet account to create (seeds: ['WALLET', authority])")]
    #[account(1, signer, name="authority", desc = "The authority of the wallet")]
    #[account(2, name="vault", desc = "The wallet vault (seeds: ['VAULT', wallet])")]
    #[account(3, writable, signer, name="payer", desc = "The account paying for the storage fees")]
    #[account(4, name="system_program", desc = "The system program")]
    Create,

    /// Proxy call
    #[account(0, name="wallet", desc = "The program derived address of the wallet account to increment (seeds: ['WALLET', authority])")]
    #[account(1, signer, name="authority", desc = "The authority of the wallet")]
    #[account(2, name="vault", desc = "The wallet vault (seeds: ['VAULT', wallet])")]
    #[account(3, name="target_program", desc = "The proxy called program")]
    ProxyCall,
}
