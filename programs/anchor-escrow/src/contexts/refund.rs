use anchor_lang::prelude::*;

use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};

use crate::Escrow;

#[derive(Accounts)]
pub struct Refund<'info> {
    #[account(mut)]
    // Signer means account must exist and be a regular wallet
    // must be in transaction signers
    // transaction must be signed by this account's private key
    maker: Signer<'info>,
    // previously needed to validate token_program to ensure consistency when creating vault
    // now escrow and vault have already stored the mint, so we check that. no need to also check token_program
    mint_a: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    maker_ata_a: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        // zero out account data, transfer all lamports to the provided account (maker)
        // and mark account as closed in the solana runtime.
        close = maker,
        // has_one basically checks that escrow.mint_a = mint_a.key()
        // escrow.maker = maker.key()
        // makes sure the refund goes to only the original maker, and escrow must be for provided token
        has_one = mint_a,
        has_one = maker,
        seeds = [b"escrow", maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump
    )]
    escrow: Account<'info, Escrow>,
    // ensuring this is an associated token account with the following characteristics
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    // 3 standard hard-coded programs that do necessary tasks
    // associated_token_program generates the default account for a pair of (holder, token)
    // token program creates new token mints
    // system program handles user accounts
    associated_token_program: Program<'info, AssociatedToken>,
    token_program: Interface<'info, TokenInterface>,
    system_program: Program<'info, System>,
}

impl<'info> Refund<'info> {
    pub fn refund_and_close_vault(&mut self) -> Result<()> {
        // PDAs don't have a private key to sign transactions
        // so they need to sign by providing their seeds
        // Solana runtime checks that the provided seeds create the PDA...
        // that is trying to execute the transaction.
        // explain this data structure
        let signer_seeds: [&[&[u8]]; 1] = [&[
            b"escrow",
            self.maker.to_account_info().key.as_ref(),
            &self.escrow.seed.to_le_bytes()[..],
            &[self.escrow.bump],
        ]];
        // struct for transfer_checked call and CPI
        // performs various safety checks such as the from account belonging to the authority
        // and the token mint and of the account matching
        let xfer_accounts = TransferChecked {
            from: self.vault.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: self.maker_ata_a.to_account_info(),
            authority: self.escrow.to_account_info(),
        };

        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            xfer_accounts,
            &signer_seeds,
        );

        transfer_checked(ctx, self.vault.amount, self.mint_a.decimals)?;

        // close the vault account
        // need to do this as a CPI because the associated token program owns vault
        // while you own escrow, so you can just close it with the inherited attribute
        // (which is much easier and automatically transfers the tokens, it knows you have authority)
        let close_accounts = CloseAccount {
            // the account to close (must have 0 tokens)
            // where to send the SOL for rent 
            // the authority to close the account
            account: self.vault.to_account_info(),
            destination: self.maker.to_account_info(),
            authority: self.escrow.to_account_info(),
        };

        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            close_accounts,
            &signer_seeds,
        );

        close_account(ctx)
    }
}
