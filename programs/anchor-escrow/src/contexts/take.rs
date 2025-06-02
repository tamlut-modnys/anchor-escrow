use anchor_lang::prelude::*;

use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{close_account, transfer_checked, Mint, TokenAccount, TokenInterface, CloseAccount, TransferChecked},
};

use crate::Escrow;

#[derive(Accounts)]
pub struct Take<'info> {
    // Signer means account must exist and be a regular wallet
    // must be in transaction signers
    // transaction must be signed by this account's private key
    #[account(mut)]
    pub taker: Signer<'info>,
    // However SystemAccount just means has to exist and be a regular wallet
    // maker is totally passive here
    #[account(mut)]
    pub maker: SystemAccount<'info>,
    pub mint_a: InterfaceAccount<'info, Mint>,
    pub mint_b: InterfaceAccount<'info, Mint>,
    // create an account (if needed) for taker to receive the tokens
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_a,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_ata_a: Box<InterfaceAccount<'info, TokenAccount>>,
    // no init since takers account for token b should already exist
    // mut since we are deducting token b from taker
    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,
    // create an account if needed for maker to receive token b in exchange
    // taker also pays for this account to be created.
    // this ensures the transaction goes through
    // maker can't pay since they aren't signing this transaction.
    // we don't want to force maker to be online and sign for this transaction to occur, would defeat escrow purpose.
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_b,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,
    // mut since we withdraw and close
    // close = maker to refund rent to maker
    // check that the maker, token a, and token b of the escrow match the transaction
    // uses seeds and bump to verify the transaction initiator provided the correct escrow account
    // the seeds and bump create the constraint for the provided maker account and the escrow
    // to fit each other
    #[account(
        mut,
        close = maker,
        has_one = maker,
        has_one = mint_a,
        has_one = mint_b,
        seeds = [b"escrow", maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump
    )]
    escrow: Account<'info, Escrow>,
    // although we don't own the vault account (the associated token program does)
    // we can still mark it as mut to say this instruction needs write access to this acc,
    // this account will be modified during execution
    // mut doesn't necessarily mean our program owns this acccount or can write directly to it.
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Take<'info> {
    // &mut self means a self transformation on the set of provided accounts

    //moves token b from taker's token b account to maker's
    pub fn deposit(&mut self) -> Result<()> {
        let transfer_accounts = TransferChecked {
            from: self.taker_ata_b.to_account_info(),
            mint: self.mint_b.to_account_info(),
            to: self.maker_ata_b.to_account_info(),
            authority: self.taker.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(self.token_program.to_account_info(), transfer_accounts);

        transfer_checked(cpi_ctx, self.escrow.receive, self.mint_b.decimals)
    }

    // move token a from vault to taker's token a account
    // logic is basically same as refund just with different target
    // (double check?)
    pub fn withdraw_and_close_vault(&mut self) -> Result<()> {
        // signing on behalf of the escrow account
        // makes sure it's the right account corresponding to the maker
        let signer_seeds: [&[&[u8]]; 1] = [&[
            b"escrow",
            self.maker.to_account_info().key.as_ref(),
            &self.escrow.seed.to_le_bytes()[..],
            &[self.escrow.bump],
        ]];

        let accounts = TransferChecked {
            from: self.vault.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: self.taker_ata_a.to_account_info(),
            authority: self.escrow.to_account_info(),
        };

        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            accounts,
            &signer_seeds,
        );

        transfer_checked(ctx, self.vault.amount, self.mint_a.decimals)?;

        let accounts = CloseAccount {
            account: self.vault.to_account_info(),
            destination: self.taker.to_account_info(),
            authority: self.escrow.to_account_info(),
        };

        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            accounts,
            &signer_seeds,
        );

        close_account(ctx)
    }
}
