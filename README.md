# Token Tax Collection and Swap Program

A Solana smart contract built with Anchor that implements token tax collection and automated swapping using Raydium's CPMM pools.

## Features

- **Tax Collection**: Collects a configurable tax (default 5%) on token transfers
- **Automated Swapping**: Swaps collected tokens for SOL using Raydium
- **Token-2022 Support**: Full support for Token-2022 extensions
- **Admin Controls**: 
  - Fee updates
  - Emergency withdrawals
  - Program pause/unpause
- **Security Features**:
  - Input validation
  - Error handling
  - Event logging
  - Access control

## Prerequisites

- Rust 1.70.0 or later
- Solana Tool Suite 1.16.0 or later
- Anchor Framework 0.30.0 or later
- Node.js 16.0.0 or later

## Installation

1. Clone the repository:
```bash
git clone 
cd 
```

2. Install dependencies:
```bash
yarn install
```

3. Build the program:
```bash
anchor build
```

## Program Accounts

### ProgramState
- Admin public key
- Fee basis points
- Pause status

### TaxWallet (PDA)
- Holds collected tax tokens
- Seed: "tax_wallet"

## Instructions

### 1. Initialize
```typescript
await program.methods
  .initialize()
  .accounts({
    state: statePDA,
    admin: adminWallet.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .rpc();
```

### 2. Collect Tax
```typescript
await program.methods
  .collectTax(new BN(amount))
  .accounts({
    state: statePDA,
    user: userWallet.publicKey,
    userTokenAccount: userATA,
    taxWallet: taxWalletPDA,
    tokenMint: tokenMint,
    tokenProgram: TOKEN_2022_PROGRAM_ID,
    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  })
  .rpc();
```

### 3. Swap Tokens for SOL
```typescript
await program.methods
  .swapTokensForSol(
    new BN(minimumSolAmount),
    new BN(deadline)
  )
  .accounts({
    state: statePDA,
    taxWallet: taxWalletPDA,
    ammId: ammId,
    ammAuthority: ammAuthority,
    // ... other required accounts
  })
  .rpc();
```

### 4. Admin Functions
```typescript
// Update fee
await program.methods
  .updateFee(new BN(newFeeBps))
  .accounts({
    state: statePDA,
    admin: adminWallet.publicKey,
  })
  .rpc();

// Toggle pause
await program.methods
  .togglePause()
  .accounts({
    state: statePDA,
    admin: adminWallet.publicKey,
  })
  .rpc();

// Emergency withdraw
await program.methods
  .emergencyWithdraw(new BN(amount))
  .accounts({
    state: statePDA,
    admin: adminWallet.publicKey,
    taxWallet: taxWalletPDA,
    adminTokenAccount: adminATA,
  })
  .rpc();
```

## Events

- ProgramInitialized
- FeeUpdated
- ProgramPauseToggled
- TaxCollected
- SwapCompleted
- EmergencyWithdraw

## Error Codes

- InvalidAmount
- AmountOverflow
- InvalidFee
- SwapExpired
- ProgramPaused
- InsufficientBalance

## Security Considerations

1. Fee Limits
   - Maximum fee: 10% (1000 basis points)
   - Minimum transfer: 1000 tokens

2. Access Control
   - Admin-only functions for critical operations
   - PDA-based tax wallet

3. Safety Checks
   - Amount validation
   - Deadline checks for swaps
   - Balance verification

## Testing

Run the test suite:
```bash
anchor test
```

## Deployment

1. Configure your Anchor.toml:
```toml
[programs.devnet]
swap = "your_program_id"
```

2. Deploy to devnet:
```bash
anchor deploy --provider.cluster devnet
```

3. Initialize the program:
```bash
anchor run initialize
```