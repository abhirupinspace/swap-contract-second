# Token to SOL Swap Program

A Solana smart contract that swaps Token-2022 tokens for SOL using Raydium's CPMM (Constant Product Market Maker) pools.

## Features
- Swap Token-2022 tokens for SOL
- Uses Raydium liquidity pools
- Slippage protection
- Event logging

## Contract Address
- Program ID: `9qxgVVgdrRCTP6BvYrDePWhk9FV5gxzggp79HDo4xkwo`
- Raydium Program ID: `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8`

## Setup & Installation

```bash
# Install dependencies
npm install

# Build the program
anchor build

# Deploy
anchor deploy
```

## Usage

### Initialize Swap Wallet
```typescript
// Create Token-2022 account for the swap wallet PDA
const [swapWalletPDA] = await PublicKey.findProgramAddress(
    [Buffer.from("swap_wallet")],
    programId
);

await createAssociatedTokenAccount(
    connection,
    payer,
    tokenMint,
    swapWalletPDA,
    TOKEN_2022_PROGRAM_ID
);
```

### Execute Swap
```typescript
await program.methods
    .swapTokensForSol(
        new BN(amount),           // Amount of tokens to swap
        new BN(minimumSolAmount)  // Minimum SOL to receive
    )
    .accounts({
        user: wallet.publicKey,
        tokenAccount: userTokenAccount,
        tokenMint: tokenMint,
        swapWallet: swapWalletPDA,
        ammId: ammId,
        ammAuthority: ammAuthority,
        sourceInfo: sourceInfo,
        destinationInfo: destinationInfo,
        poolTokenCoinAccount: poolCoinAccount,
        poolTokenPcAccount: poolPcAccount,
        serumProgramId: serumProgramId,
        serumMarket: serumMarket,
        serumBids: serumBids,
        serumAsks: serumAsks,
        serumEventQueue: serumEventQueue,
        serumCoinVaultAccount: serumCoinVault,
        serumPcVaultAccount: serumPcVault,
        serumVaultSigner: serumVaultSigner,
        receiver: receiverAccount,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
    })
    .rpc();
```

## Required Accounts

### User Accounts
- `user`: Signer executing the swap
- `tokenAccount`: User's token account (source of tokens)
- `tokenMint`: Token mint address
- `receiver`: Account to receive SOL

### Program PDAs
- `swapWallet`: Program's token account (PDA)

### Raydium Pool Accounts
- `ammId`: Raydium pool address
- `ammAuthority`: Pool authority
- `sourceInfo`: Source token pool info
- `destinationInfo`: Destination token pool info
- `poolTokenCoinAccount`: Pool coin token account
- `poolTokenPcAccount`: Pool PC token account

### Serum Market Accounts
- `serumProgramId`: Serum DEX program
- `serumMarket`: Market address
- `serumBids`: Bids account
- `serumAsks`: Asks account
- `serumEventQueue`: Event queue
- `serumCoinVaultAccount`: Coin vault
- `serumPcVaultAccount`: PC vault
- `serumVaultSigner`: Vault signer

### System Accounts
- `tokenProgram`: Token-2022 Program
- `associatedTokenProgram`: Associated Token Program
- `systemProgram`: System Program

## Events

### SwapCompleted
```typescript
{
    tokenAmount: u64,    // Amount of tokens swapped
    solAmount: u64,      // Amount of SOL received
    receiver: PublicKey, // Receiver's address
    timestamp: i64       // Timestamp of swap
}
```

## Error Codes

- `InvalidAmount`: Amount must be greater than 0

## Security Considerations

1. **Slippage Protection**
   - Use `minimum_sol_amount` to protect against unfavorable swaps
   - Calculate appropriate slippage based on market conditions

2. **Account Validation**
   - All Raydium and Serum accounts must match the pool
   - Token accounts must belong to correct owners

3. **Token Program**
   - Contract uses Token-2022 program
   - Ensure token accounts are created with correct program

## Development

### Testing
```bash
anchor test
```

### Building
```bash
anchor build
```

### Deploying
```bash
anchor deploy
```