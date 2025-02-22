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

// Program Constants
pub const RAYDIUM_PROGRAM_ID: Pubkey = pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
pub const TAX_WALLET_SEED: &[u8] = b"tax_wallet";
pub const ADMIN_SEED: &[u8] = b"admin";
pub const MAX_FEE_BPS: u16 = 1000; // 10%
pub const MIN_TRANSFER_AMOUNT: u64 = 1000;
pub const SWAP_DEADLINE_EXTENSION: i64 = 900; // 15 minutes

#[program]
pub mod second_swap {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.admin = ctx.accounts.admin.key();
        state.fee_basis_points = 500; // 5% default fee
        state.paused = false;
        emit!(ProgramInitialized {
            admin: ctx.accounts.admin.key(),
            fee_basis_points: state.fee_basis_points,
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }

    pub fn update_fee(ctx: Context<AdminOnly>, new_fee_bps: u16) -> Result<()> {
        require!(new_fee_bps <= MAX_FEE_BPS, ErrorCode::InvalidFee);
        let state = &mut ctx.accounts.state;
        state.fee_basis_points = new_fee_bps;
        emit!(FeeUpdated {
            old_fee: ctx.accounts.state.fee_basis_points,
            new_fee: new_fee_bps,
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }

    pub fn toggle_pause(ctx: Context<AdminOnly>) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.paused = !state.paused;
        emit!(ProgramPauseToggled {
            paused: state.paused,
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }

    pub fn collect_tax(ctx: Context<CollectTax>, amount: u64) -> Result<()> {
        let state = &ctx.accounts.state;
        require!(!state.paused, ErrorCode::ProgramPaused);
        require!(amount >= MIN_TRANSFER_AMOUNT, ErrorCode::InvalidAmount);

        let fee_amount = (amount as u128 * state.fee_basis_points as u128 / 10000) as u64;
        let net_amount = amount.checked_sub(fee_amount)
            .ok_or(ErrorCode::AmountOverflow)?;

        let cpi_accounts = token_2022::TransferChecked {
            from: ctx.accounts.user_token_account.to_account_info(),
            mint: ctx.accounts.token_mint.to_account_info(),
            to: ctx.accounts.tax_wallet.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        );

        token_2022::transfer_checked(cpi_ctx, amount, ctx.accounts.token_mint.decimals)?;

        emit!(TaxCollected {
            amount,
            fee_amount,
            net_amount,
            user: ctx.accounts.user.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }

    pub fn swap_tokens_for_sol(
        ctx: Context<SwapTokensForSol>,
        minimum_sol_amount: u64,
        deadline: i64,
    ) -> Result<()> {
        let state = &ctx.accounts.state;
        require!(!state.paused, ErrorCode::ProgramPaused);
        require!(minimum_sol_amount > 0, ErrorCode::InvalidAmount);
        
        let current_timestamp = Clock::get()?.unix_timestamp;
        require!(current_timestamp <= deadline, ErrorCode::SwapExpired);

        let (tax_wallet_pda, bump) = Pubkey::find_program_address(
            &[TAX_WALLET_SEED],
            ctx.program_id,
        );
        let signer_seeds = &[TAX_WALLET_SEED, &[bump]];

        let token_balance = ctx.accounts.tax_wallet.amount;
        require!(token_balance > 0, ErrorCode::InsufficientBalance);

        let swap_instruction_data = RaydiumSwapInstruction {
            instruction: 9,
            amount_in: token_balance,
            minimum_amount_out: minimum_sol_amount,
        };

        // Create swap instruction
        let swap_ix = Instruction {
            program_id: RAYDIUM_PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(ctx.accounts.amm_id.key(), false),
                AccountMeta::new(ctx.accounts.amm_authority.key(), false),
                AccountMeta::new(tax_wallet_pda, true),
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
                ctx.accounts.tax_wallet.to_account_info(),
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
        let sol_balance = **ctx.accounts.tax_wallet.to_account_info().lamports.borrow();
        let transfer_ix = system_instruction::transfer(
            &tax_wallet_pda,
            &ctx.accounts.receiver.key(),
            sol_balance,
        );

        invoke_signed(
            &transfer_ix,
            &[
                ctx.accounts.tax_wallet.to_account_info(),
                ctx.accounts.receiver.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[signer_seeds],
        )?;

        emit!(SwapCompleted {
            token_amount: token_balance,
            sol_amount: sol_balance,
            receiver: ctx.accounts.receiver.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }

    pub fn emergency_withdraw(ctx: Context<AdminOnly>, amount: u64) -> Result<()> {
        require!(amount > 0, ErrorCode::InvalidAmount);
        let (tax_wallet_pda, bump) = Pubkey::find_program_address(
            &[TAX_WALLET_SEED],
            ctx.program_id,
        );
        let signer_seeds = &[TAX_WALLET_SEED, &[bump]];

        // Transfer tokens to admin
        let cpi_accounts = token_2022::Transfer {
            from: ctx.accounts.tax_wallet.to_account_info(),
            to: ctx.accounts.admin_token_account.to_account_info(),
            authority: ctx.accounts.tax_wallet.to_account_info(),
        };

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            &[signer_seeds],
        );

        token_2022::transfer(cpi_ctx, amount)?;

        emit!(EmergencyWithdraw {
            amount,
            admin: ctx.accounts.admin.key(),
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

#[account]
pub struct ProgramState {
    pub admin: Pubkey,
    pub fee_basis_points: u16,
    pub paused: bool,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid amount provided")]
    InvalidAmount,
    #[msg("Amount calculation overflow")]
    AmountOverflow,
    #[msg("Fee exceeds maximum allowed")]
    InvalidFee,
    #[msg("Swap deadline expired")]
    SwapExpired,
    #[msg("Program is paused")]
    ProgramPaused,
    #[msg("Insufficient balance")]
    InsufficientBalance,
}

#[event]
pub struct ProgramInitialized {
    pub admin: Pubkey,
    pub fee_basis_points: u16,
    pub timestamp: i64,
}

#[event]
pub struct FeeUpdated {
    pub old_fee: u16,
    pub new_fee: u16,
    pub timestamp: i64,
}

#[event]
pub struct ProgramPauseToggled {
    pub paused: bool,
    pub timestamp: i64,
}

#[event]
pub struct TaxCollected {
    pub amount: u64,
    pub fee_amount: u64,
    pub net_amount: u64,
    pub user: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct SwapCompleted {
    pub token_amount: u64,
    pub sol_amount: u64,
    pub receiver: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct EmergencyWithdraw {
    pub amount: u64,
    pub admin: Pubkey,
    pub timestamp: i64,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 2 + 1,
        seeds = [b"state"],
        bump
    )]
    pub state: Account<'info, ProgramState>,
    
    #[account(mut)]
    pub admin: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AdminOnly<'info> {
    #[account(
        mut,
        seeds = [b"state"],
        bump,
        has_one = admin,
    )]
    pub state: Account<'info, ProgramState>,
    
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(mut)]
    pub tax_wallet: Box<Account<'info, token_2022::TokenAccount>>,
    
    #[account(mut)]
    pub admin_token_account: Box<Account<'info, token_2022::TokenAccount>>,
    
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CollectTax<'info> {
    #[account(
        seeds = [b"state"],
        bump
    )]
    pub state: Account<'info, ProgramState>,

    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(
        mut,
        constraint = user_token_account.owner == user.key()
    )]
    pub user_token_account: Box<Account<'info, token_2022::TokenAccount>>,
    
    #[account(
        mut,
        seeds = [TAX_WALLET_SEED],
        bump,
    )]
    pub tax_wallet: Box<Account<'info, token_2022::TokenAccount>>,
    
    pub token_mint: Box<Account<'info, token_2022::Mint>>,
    
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SwapTokensForSol<'info> {
    #[account(
        seeds = [b"state"],
        bump
    )]
    pub state: Account<'info, ProgramState>,

    #[account(
        mut,
        seeds = [TAX_WALLET_SEED],
        bump,
    )]
    pub tax_wallet: Box<Account<'info, token_2022::TokenAccount>>,
    
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
    pub system_program: Program<'info, System>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::solana_program::clock::Clock;
    
    #[test]
    fn test_initialize() {
        // Mock test environment
        let mut lamports = 0;
        let mut data = vec![0; 32];
        let owner = Pubkey::new_unique();
        let admin = Pubkey::new_unique();
        
        let state_account_info = AccountInfo::new(
            &owner,
            false,
            true,
            &mut lamports,
            &mut data,
            &owner,
            false,
            0,
        );
        
        let admin_account_info = AccountInfo::new(
            &admin,
            true,
            false,
            &mut lamports,
            &mut data,
            &owner,
            false,
            0,
        );
        
        let system_program_info = AccountInfo::new(
            &system_program::ID,
            false,
            false,
            &mut lamports,
            &mut data,
            &owner,
            false,
            0,
        );
        
        let initialize_accounts = Initialize {
            state: Account::try_from(&state_account_info).unwrap(),
            admin: Signer::try_from(&admin_account_info).unwrap(),
            system_program: Program::try_from(&system_program_info).unwrap(),
        };
        
        let context = Context::new(
            Pubkey::new_unique(),
            initialize_accounts,
            &[],
            BTreeMap::new(),
        );
        
        let result = initialize(context);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_tax_calculation() {
        let amount: u64 = 1_000_000;
        let fee_basis_points: u16 = 500; // 5%
        
        let fee_amount = (amount as u128 * fee_basis_points as u128 / 10000) as u64;
        let net_amount = amount.checked_sub(fee_amount).unwrap();
        
        assert_eq!(fee_amount, 50_000);
        assert_eq!(net_amount, 950_000);
    }
    
    #[test]
    fn test_swap_validation() {
        let minimum_sol_amount: u64 = 1_000_000;
        let deadline = Clock::get().unwrap().unix_timestamp + SWAP_DEADLINE_EXTENSION;
        
        assert!(minimum_sol_amount > 0);
        assert!(Clock::get().unwrap().unix_timestamp <= deadline);
    }
    
    #[test]
    fn test_fee_update() {
        let new_fee: u16 = 600;
        assert!(new_fee <= MAX_FEE_BPS);
    }
    
    #[test]
    #[should_panic]
    fn test_invalid_fee() {
        let new_fee: u16 = 1500; // 15% > MAX_FEE_BPS
        assert!(new_fee <= MAX_FEE_BPS);
    }
}