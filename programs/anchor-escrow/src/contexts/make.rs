use anchor_lang::prelude::*;

use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

use crate::Escrow;

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Make<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,
    #[account(
        mint::token_program = token_program
    )]
    // InterfaceAccount deserializes the info into a Mint data structure
    pub mint_a: InterfaceAccount<'info, Mint>,
    #[account(
        mint::token_program = token_program
    )]
    pub mint_b: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    pub maker_ata_a: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init,
        payer = maker,
        space = 8 + Escrow::INIT_SPACE,
        seeds = [b"escrow", maker.key().as_ref(), seed.to_le_bytes().as_ref()],
        bump
    )]
    pub escrow: Account<'info, Escrow>,
    // creating a new associated token account with the following characteristics
    // vault is also a PDA but anchor and the associated token program hide some details
    // (no seeds or bump required)
    #[account(
        init,
        payer = maker,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    // 3 standard hard-coded programs that do necessary tasks
    // associated_token_program generates the default account for a pair of (holder, token)
    // token program creates new token mints
    // system program handles user accounts
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Make<'info> {
    pub fn save_escrow(&mut self, seed: u64, receive: u64, bumps: &MakeBumps) -> Result<()> {
        // set_inner is an anchor method that replaces the entire content of an account
        self.escrow.set_inner(Escrow {
            seed,
            maker: self.maker.key(),
            mint_a: self.mint_a.key(),
            mint_b: self.mint_b.key(),
            receive,
            bump: bumps.escrow,
        });
        Ok(())
    }

    pub fn deposit(&mut self, deposit: u64) -> Result<()> {
        // struct for CPI. probably checks that the authority has control over the from account.
        let transfer_accounts = TransferChecked {
            // to_account_info converts anchors typed InterfaceAccount<'info, TokenAccount> to raw AccountInfo<'info>
            from: self.maker_ata_a.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.maker.to_account_info(),
        };
        // first arg: program we're calling, second arg: accounts that program needs
        // cpi is how we invoke other programs on the solana blockchain.
        // cpi context ensures we have all the necessary info to do so.
        let cpi_ctx = CpiContext::new(self.token_program.to_account_info(), transfer_accounts);

        // actual transfer action occurs here
        // transfer_checked needs the decimals of the token as last arg as a safety measure
        // it's an explicit declarataion of intent, not necessary as this is transferred in the cpi in mint
        // however using self.mint_a.decimals makes it useless
        transfer_checked(cpi_ctx, deposit, self.mint_a.decimals)

        // standard 3 step pattern for CPI calls in Anchor:
        // make accounts struct, create context, then perform the transfer

        /*The Token Program validates:
        Decimal Match: mint.decimals == provided_decimals
        Mint Match: Both accounts belong to the same mint
        Authority: Authority has permission to move funds
        Sufficient Balance: Source has enough tokens
        No Overflow: Destination won't overflow */
    }
}
