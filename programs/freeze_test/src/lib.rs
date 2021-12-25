use anchor_lang::prelude::*;
use anchor_spl::{associated_token, token};
use spl_token;
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

const MEMBERSHIP_SEED: &[u8] = b"member";
const AUTHORITY_SEED: &[u8] = b"authority";

#[program]
pub mod freeze_test {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, authority_bump: u8) -> ProgramResult {
        ctx.accounts.governance_mint_authority.bump = authority_bump;
        Ok(())
    }

    pub fn create_membership(ctx: Context<CreateMembership>, membership_bump: u8) -> ProgramResult {
        ctx.accounts.membership.authority = ctx.accounts.authority.key();
        ctx.accounts.membership.bump = membership_bump;

        let mint_authority_seeds = &[
            AUTHORITY_SEED,
            &[ctx.accounts.governance_mint_authority.bump],
        ];
        token::mint_to(
            ctx.accounts
                .into_mint_gov_tokens_to_new_member_context()
                .with_signer(&[mint_authority_seeds]),
            100,
        )?;
        Ok(())
    }
    pub fn claim_membership(ctx: Context<ClaimMembership>) -> ProgramResult {
        ctx.accounts.membership.authority = ctx.accounts.claimant.key();
        let mint_authority_seeds = &[
            AUTHORITY_SEED,
            &[ctx.accounts.governance_mint_authority.bump],
        ];
        //mint gov tokens to claimant
        token::mint_to(
            ctx.accounts
                .into_mint_gov_tokens_to_claimant_context()
                .with_signer(&[mint_authority_seeds]),
            100,
        )?;
        //change membership authority
        ctx.accounts.membership.authority = ctx.accounts.claimant.key();
        //freeze old member token account
        token::freeze_account(
            ctx.accounts
                .into_freeze_old_member_token_account_context()
                .with_signer(&[mint_authority_seeds]),
        )?;

        //later
        //set data for new membership attribution
        //close old membership attribution
        Ok(())
    }

    pub fn thaw_governance_token_account(
        ctx: Context<ThawGovernanceTokenAccount>,
    ) -> ProgramResult {
        //burn tokens from the account
        //thaw it
        //this will work right?
        //it should. one safety mech i could have is just remove balance up to amount that u get when u mint
        //so u wouldn't get totally rugged if u had more. cool
        let mint_authority_seeds = &[
            AUTHORITY_SEED,
            &[ctx.accounts.governance_mint_authority.bump],
        ];
        token::thaw_account(
            ctx.accounts
                .into_thaw_token_account_context()
                .with_signer(&[mint_authority_seeds]),
        )?;
        token::burn(ctx.accounts.into_burn_gov_tokens_context(), 100)?;
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(authority_bump: u8)]
pub struct Initialize<'info> {
    #[account(mut)]
    initializer: Signer<'info>,
    #[account(
        init,
        seeds = [AUTHORITY_SEED],
        bump = authority_bump,
        payer = initializer
    )]
    governance_mint_authority: Account<'info, MintAuthority>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(membership_bump: u8)]
pub struct CreateMembership<'info> {
    #[account(mut)]
    authority: Signer<'info>,
    #[account(
        init,
        seeds = [MEMBERSHIP_SEED, authority.key().as_ref()], //add a seed for the auth key, converted to card mint later?
        bump = membership_bump,
        payer = authority,
    )]
    membership: Account<'info, Membership>,
    #[account(mut)]
    governance_token_account: Account<'info, token::TokenAccount>,
    #[account(mut)]
    governance_mint: Account<'info, token::Mint>,
    governance_mint_authority: Account<'info, MintAuthority>,
    token_program: Program<'info, token::Token>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimMembership<'info> {
    claimant: Signer<'info>,
    #[account(mut)]
    membership: Account<'info, Membership>,
    #[account(mut)]
    governance_mint: Account<'info, token::Mint>,
    governance_mint_authority: Account<'info, MintAuthority>,
    #[account(
        mut,
        address = associated_token::get_associated_token_address(&claimant.key(), &governance_mint.key())
    )]
    claimant_token_account: Account<'info, token::TokenAccount>,
    #[account(
        mut,
        address = associated_token::get_associated_token_address(&membership.authority, &governance_mint.key())
    )]
    old_member_token_account: Account<'info, token::TokenAccount>,
    token_program: Program<'info, token::Token>,
}

//if u sell a membership, when the receiver claims, your gov token account will be frozen
//to thaw it, u have to call this, which burns the amount u received when u created the membership
#[derive(Accounts)]
pub struct ThawGovernanceTokenAccount<'info> {
    #[account(mut)]
    token_account_owner: Signer<'info>,
    #[account(
        mut,
        address = associated_token::get_associated_token_address(&token_account_owner.key(), &governance_mint.key()),
        constraint = token_account.state == spl_token::state::AccountState::Frozen,
        constraint = token_account.owner == token_account_owner.key()
    )]
    token_account: Account<'info, token::TokenAccount>,
    #[account(mut)]
    governance_mint: Account<'info, token::Mint>,
    governance_mint_authority: Account<'info, MintAuthority>,
    burner: Account<'info, token::TokenAccount>,
    token_program: Program<'info, token::Token>,
}

#[account]
#[derive(Default, Copy)]
pub struct Membership {
    authority: Pubkey,
    bump: u8,
}

#[account]
#[derive(Default)]
pub struct MintAuthority {
    bump: u8,
}

impl<'info> CreateMembership<'info> {
    pub fn into_mint_gov_tokens_to_new_member_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, token::MintTo<'info>> {
        let cpi_program = self.token_program.to_account_info();
        let cpi_context = token::MintTo {
            mint: self.governance_mint.to_account_info(),
            to: self.governance_token_account.to_account_info(),
            authority: self.governance_mint_authority.to_account_info(),
        };
        CpiContext::new(cpi_program, cpi_context)
    }
}

impl<'info> ClaimMembership<'info> {
    pub fn into_freeze_old_member_token_account_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, token::FreezeAccount<'info>> {
        let cpi_program = self.token_program.to_account_info();
        let cpi_context = token::FreezeAccount {
            account: self.old_member_token_account.to_account_info(),
            mint: self.governance_mint.to_account_info(),
            authority: self.governance_mint_authority.to_account_info(),
        };
        CpiContext::new(cpi_program, cpi_context)
    }
    pub fn into_mint_gov_tokens_to_claimant_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, token::MintTo<'info>> {
        let cpi_program = self.token_program.to_account_info();
        let cpi_context = token::MintTo {
            mint: self.governance_mint.to_account_info(),
            to: self.claimant_token_account.to_account_info(),
            authority: self.governance_mint_authority.to_account_info(),
        };
        CpiContext::new(cpi_program, cpi_context)
    }
}

impl<'info> ThawGovernanceTokenAccount<'info> {
    pub fn into_thaw_token_account_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, token::ThawAccount<'info>> {
        let cpi_program = self.token_program.to_account_info();
        let cpi_context = token::ThawAccount {
            account: self.token_account.to_account_info(),
            mint: self.governance_mint.to_account_info(),
            authority: self.governance_mint_authority.to_account_info(),
        };
        CpiContext::new(cpi_program, cpi_context)
    }
    pub fn into_burn_gov_tokens_context(
        &self,
    ) -> CpiContext<'_, '_, '_, 'info, token::Burn<'info>> {
        let cpi_program = self.token_program.to_account_info();
        let cpi_context = token::Burn {
            to: self.token_account.to_account_info(),
            mint: self.governance_mint.to_account_info(),
            authority: self.token_account_owner.to_account_info(),
        };
        CpiContext::new(cpi_program, cpi_context)
    }
}

// fn associated_token_account_address(mint: Pubkey, owner: Pubkey) -> Pubkey {
//     const TOKEN_PROGRAM_ID = token::id();
// }

//i already enforce the authority card_mint match anyway so maybe i should just switch it to wallet address
//then i wouldn't need the attribution. could be nice
//i don't think i would really lose anything with that. would i? will revisit

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
