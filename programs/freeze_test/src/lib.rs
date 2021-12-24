use anchor_lang::prelude::*;
use anchor_spl::{associated_token, token};
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

const MEMBERSHIP_SEED: &[u8] = b"member";

#[program]
pub mod freeze_test {
    use super::*;
    pub fn create_membership(ctx: Context<CreateMembership>, membership_bump: u8) -> ProgramResult {
        ctx.accounts.membership.authority = ctx.accounts.creator.key();
        ctx.accounts.membership.bump = membership_bump;
        Ok(())
    }
    pub fn claim_membership(ctx: Context<ClaimMembership>) -> ProgramResult {
        ctx.accounts.membership.authority = ctx.accounts.claimant.key();
        //freeze old token account
        //mint new gov tokens to new member

        Ok(())
    }

    pub fn thaw_governance_token_account(
        ctx: Context<ThawGovernanceTokenAccount>,
    ) -> ProgramResult {
        //drain the account
        //thaw it
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(membership_bump: u8)]
pub struct CreateMembership<'info> {
    #[account(mut)]
    creator: Signer<'info>,
    #[account(
        init,
        seeds = [MEMBERSHIP_SEED],
        bump = membership_bump,
        payer = creator,
    )]
    membership: Account<'info, Membership>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimMembership<'info> {
    claimant: Signer<'info>,
    membership: Account<'info, Membership>,
    governance_mint: Account<'info, token::Mint>,
    #[account(
        address = associated_token::get_associated_token_address(&claimant.key(), &governance_mint.key())
    )]
    claimant_token_account: Account<'info, token::TokenAccount>,
    #[account(
        address = associated_token::get_associated_token_address(&membership.authority, &governance_mint.key())
    )]
    old_member_token_account: Account<'info, token::TokenAccount>,
}

#[derive(Accounts)]
pub struct ThawGovernanceTokenAccount<'info> {
    owner: Signer<'info>,
}

#[account]
#[derive(Default, Copy)]
pub struct Membership {
    authority: Pubkey,
    bump: u8,
}

// fn associated_token_account_address(mint: Pubkey, owner: Pubkey) -> Pubkey {
//     const TOKEN_PROGRAM_ID = token::id();
// }

/*
u know they can claim the memberhsip if they are holding the token for the card_mint
*/

/*
make a membership struct with a token account in it,
techinically shouldn't even need to store the token account in the membership
u are going to run into some issues if someone sells and then reclaims
i guess they will have to call a func to thaw the account first
so u can't actually transfer to a token account that is frozen
interesting

only piece that i haven't ironed out is how to make sure the right account is getting passed in


what u will probably have to do is have them call an ix that burns the balance and thaws the account in one go
whatever
*/
