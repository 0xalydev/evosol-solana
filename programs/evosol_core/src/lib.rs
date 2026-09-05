//! EvoSol Core Anchor Program
//! High-Frequency Skill Registration & Micro-Settlement for Autonomous Agents on Solana

use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("EvoSoL1111111111111111111111111111111111111");

#[program]
pub mod evosol_core {
    use super::*;

    /// Registers a newly synthesized, formally verified agent skill on-chain
    pub fn register_skill(
        ctx: Context<RegisterSkill>,
        skill_hash: [u8; 32],
        benchmark_score: u16,
        fee_lamports: u64,
        metadata_uri: String,
    ) -> Result<()> {
        require!(benchmark_score >= 8000, EvoSolError::SkillBelowQualityThreshold);
        require!(metadata_uri.len() <= 200, EvoSolError::UriTooLong);

        let skill_record = &mut ctx.accounts.skill_record;
        skill_record.author = ctx.accounts.author.key();
        skill_record.skill_hash = skill_hash;
        skill_record.benchmark_score = benchmark_score;
        skill_record.fee_lamports = fee_lamports;
        skill_record.metadata_uri = metadata_uri;
        skill_record.invocations = 0;
        skill_record.created_at = Clock::get()?.unix_timestamp;

        emit!(SkillRegisteredEvent {
            skill_id: skill_record.key(),
            author: ctx.accounts.author.key(),
            skill_hash,
            benchmark_score,
        });

        Ok(())
    }

    /// Atomically invokes a registered skill and settles the micro-payment from the calling agent to the skill author
    pub fn invoke_and_settle(ctx: Context<InvokeAndSettle>) -> Result<()> {
        let skill_record = &mut ctx.accounts.skill_record;
        let fee = skill_record.fee_lamports;

        if fee > 0 {
            let cpi_context = CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.caller.to_account_info(),
                    to: ctx.accounts.author.to_account_info(),
                },
            );
            system_program::transfer(cpi_context, fee)?;
        }

        skill_record.invocations = skill_record.invocations.checked_add(1).unwrap();

        emit!(SkillInvokedEvent {
            skill_id: skill_record.key(),
            caller: ctx.accounts.caller.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(skill_hash: [u8; 32])]
pub struct RegisterSkill<'info> {
    #[account(
        init,
        payer = author,
        space = 8 + 32 + 32 + 2 + 8 + 4 + 200 + 8 + 8,
        seeds = [b"skill", author.key().as_ref(), skill_hash.as_ref()],
        bump
    )]
    pub skill_record: Account<'info, SkillRecord>,

    #[account(mut)]
    pub author: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InvokeAndSettle<'info> {
    #[account(
        mut,
        has_one = author @ EvoSolError::InvalidAuthorAccount,
    )]
    pub skill_record: Account<'info, SkillRecord>,

    /// CHECK: Author account receives micro-royalty
    #[account(mut)]
    pub author: AccountInfo<'info>,

    #[account(mut)]
    pub caller: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct SkillRecord {
    pub author: Pubkey,
    pub skill_hash: [u8; 32],
    pub benchmark_score: u16, // basis points, e.g. 9500 = 95.00%
    pub fee_lamports: u64,
    pub metadata_uri: String,
    pub invocations: u64,
    pub created_at: i64,
}

#[event]
pub struct SkillRegisteredEvent {
    pub skill_id: Pubkey,
    pub author: Pubkey,
    pub skill_hash: [u8; 32],
    pub benchmark_score: u16,
}

#[event]
pub struct SkillInvokedEvent {
    pub skill_id: Pubkey,
    pub caller: Pubkey,
    pub timestamp: i64,
}

#[error_code]
pub enum EvoSolError {
    #[msg("Synthesized skill benchmark score does not meet the minimum verification threshold (80%).")]
    SkillBelowQualityThreshold,
    #[msg("Metadata URI string exceeds 200 character limit.")]
    UriTooLong,
    #[msg("Provided author account does not match registered skill author.")]
    InvalidAuthorAccount,
}
