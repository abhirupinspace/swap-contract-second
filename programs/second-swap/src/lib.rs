#![allow(unexpected_cfgs)]
#![cfg(all(target_arch = "bpf", not(feature = "no-entrypoint")))]

#![allow(unexpected_cfgs)]
#![cfg(all(target_arch = "bpf", not(feature = "no-entrypoint")))]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    program::invoke_signed,
    system_instruction,
    instruction::{Instruction, AccountMeta},
};
use anchor_spl::{
    token_2022::{self, Token2022},
    associated_token::AssociatedToken,
};

declare_id!("9qxgVVgdrRCTP6BvYrDePWhk9FV5gxzggp79HDo4xkwo");

// Constants
pub const RAYDIUM_PROGRAM_ID: Pubkey = pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
pub const SWAP_WALLET_SEED: &[u8] = b"swap_wallet";

#[program]
pub mod simple_swap {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }

    pub fn swap_tokens_for_sol(
        ctx: Context<SwapTokensForSol>,
        amount: u64,
        minimum_sol_amount: u64,
    ) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);
        require!(minimum_sol_amount > 0, ErrorCode::InvalidAmount);

        let (swap_wallet_pda, bump) = Pubkey::find_program_address(
            &[SWAP_WALLET_SEED],
            ctx.program_id,
        );
        let signer_seeds = &[SWAP_WALLET_SEED, &[bump]];

        // Transfer tokens to swap wallet
        let transfer_ix = token_2022::TransferChecked {
            from: ctx.accounts.token_account.to_account_info(),
            mint: ctx.accounts.token_mint.to_account_info(),
            to: ctx.accounts.swap_wallet.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };

        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            transfer_ix,
        );

        token_2022::transfer_checked(transfer_ctx, amount, ctx.accounts.token_mint.decimals)?;

        // Create Raydium swap instruction
        let swap_instruction_data = RaydiumSwapInstruction {
            instruction: 9,
            amount_in: amount,
            minimum_amount_out: minimum_sol_amount,
        };

        let swap_ix = Instruction {
            program_id: RAYDIUM_PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(ctx.accounts.amm_id.key(), false),
                AccountMeta::new(ctx.accounts.amm_authority.key(), false),
                AccountMeta::new(swap_wallet_pda, true),
                AccountMeta::new(ctx.accounts.source_info.key(), false),
                AccountMeta::new(ctx.accounts.destination_info.key(), false),
                AccountMeta::new(ctx.accounts.pool_token_coin_account.key(), false),
                AccountMeta::new(ctx.accounts.pool_token_pc_account.key(), false),
                AccountMeta::new(ctx.accounts.serum_program_id.key(), false),
                AccountMeta::new(ctx.accounts.serum_market.key(), false),
                AccountMeta::new(ctx.accounts.serum_bids.key(), false),
                AccountMeta::new(ctx.accounts.serum_asks.key(), false),
                AccountMeta::new(ctx.accounts.serum_event_queue.key(), false),
                AccountMeta::new(ctx.accounts.serum_coin_vault_account.key(), false),
                AccountMeta::new(ctx.accounts.serum_pc_vault_account.key(), false),
                AccountMeta::new(ctx.accounts.serum_vault_signer.key(), false),
                AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
            ],
            data: swap_instruction_data.try_to_vec()?,
        };

        // Execute swap
        invoke_signed(
            &swap_ix,
            &[
                ctx.accounts.amm_id.to_account_info(),
                ctx.accounts.amm_authority.to_account_info(),
                ctx.accounts.swap_wallet.to_account_info(),
                ctx.accounts.source_info.to_account_info(),
                ctx.accounts.destination_info.to_account_info(),
                ctx.accounts.pool_token_coin_account.to_account_info(),
                ctx.accounts.pool_token_pc_account.to_account_info(),
                ctx.accounts.serum_program_id.to_account_info(),
                ctx.accounts.serum_market.to_account_info(),
                ctx.accounts.serum_bids.to_account_info(),
                ctx.accounts.serum_asks.to_account_info(),
                ctx.accounts.serum_event_queue.to_account_info(),
                ctx.accounts.serum_coin_vault_account.to_account_info(),
                ctx.accounts.serum_pc_vault_account.to_account_info(),
                ctx.accounts.serum_vault_signer.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
            ],
            &[signer_seeds],
        )?;

        // Transfer SOL to receiver
        let sol_balance = **ctx.accounts.swap_wallet.to_account_info().lamports.borrow();
        let transfer_ix = system_instruction::transfer(
            &swap_wallet_pda,
            &ctx.accounts.receiver.key(),
            sol_balance,
        );

        invoke_signed(
            &transfer_ix,
            &[
                ctx.accounts.swap_wallet.to_account_info(),
                ctx.accounts.receiver.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[signer_seeds],
        )?;

        emit!(SwapCompleted {
            token_amount: amount,
            sol_amount: sol_balance,
            receiver: ctx.accounts.receiver.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct RaydiumSwapInstruction {
    pub instruction: u8,
    pub amount_in: u64,
    pub minimum_amount_out: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid amount provided")]
    InvalidAmount,
}

#[event]
pub struct SwapCompleted {
    pub token_amount: u64,
    pub sol_amount: u64,
    pub receiver: Pubkey,
    pub timestamp: i64,
}

#[derive(Accounts)]
pub struct Initialize {}

#[derive(Accounts)]
pub struct SwapTokensForSol<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// Token account containing tokens to swap
    #[account(mut)]
    pub token_account: Box<Account<'info, token_2022::TokenAccount>>,

    /// Token mint
    pub token_mint: Box<Account<'info, token_2022::Mint>>,

    /// Swap wallet PDA
    #[account(
        mut,
        seeds = [SWAP_WALLET_SEED],
        bump,
    )]
    pub swap_wallet: Box<Account<'info, token_2022::TokenAccount>>,
    
    /// CHECK: Raydium AMM account
    #[account(mut)]
    pub amm_id: UncheckedAccount<'info>,
    
    /// CHECK: AMM authority PDA
    pub amm_authority: UncheckedAccount<'info>,
    
    /// CHECK: Source token info
    #[account(mut)]
    pub source_info: Box<Account<'info, token_2022::TokenAccount>>,
    
    /// CHECK: Destination token info
    #[account(mut)]
    pub destination_info: Box<Account<'info, token_2022::TokenAccount>>,
    
    /// CHECK: Pool coin token account
    #[account(mut)]
    pub pool_token_coin_account: Box<Account<'info, token_2022::TokenAccount>>,
    
    /// CHECK: Pool pc token account
    #[account(mut)]
    pub pool_token_pc_account: Box<Account<'info, token_2022::TokenAccount>>,
    
    /// CHECK: Serum program ID
    pub serum_program_id: UncheckedAccount<'info>,
    
    /// CHECK: Serum market
    #[account(mut)]
    pub serum_market: UncheckedAccount<'info>,
    
    /// CHECK: Serum bids
    #[account(mut)]
    pub serum_bids: UncheckedAccount<'info>,
    
    /// CHECK: Serum asks
    #[account(mut)]
    pub serum_asks: UncheckedAccount<'info>,
    
    /// CHECK: Serum event queue
    #[account(mut)]
    pub serum_event_queue: UncheckedAccount<'info>,
    
    /// CHECK: Serum coin vault
    #[account(mut)]
    pub serum_coin_vault_account: UncheckedAccount<'info>,
    
    /// CHECK: Serum pc vault
    #[account(mut)]
    pub serum_pc_vault_account: UncheckedAccount<'info>,
    
    /// CHECK: Serum vault signer
    pub serum_vault_signer: UncheckedAccount<'info>,
    
    #[account(mut)]
    pub receiver: SystemAccount<'info>,
    
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}